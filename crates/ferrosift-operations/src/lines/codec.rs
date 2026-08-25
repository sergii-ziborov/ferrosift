use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use crate::jscompat::delim::{char_rep, is_js_whitespace};

const INVALID_DELIMITER: &str = "data.tail.invalid_delimiter";
const INVALID_POSITION: &str = "text.pad_lines.invalid_position";

/// Keeps the last n delimited fields, like UNIX `tail`.
///
/// A negative count drops the first `-n` fields instead, matching the
/// reference's sign convention rather than clamping.
pub(super) fn tail(
    input: &str,
    delimiter_token: &str,
    number: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let parts: Vec<&str> = split_js(input, delimiter);
    let total = i128::try_from(parts.len()).unwrap_or(i128::MAX);
    let mut kept = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let position = i128::try_from(index + 1).unwrap_or(i128::MAX);
        let keep = if number < 0 {
            position > -number
        } else {
            position > total - number
        };
        if keep {
            kept.push(*part);
        }
    }
    context.ensure_active()?;
    Ok(kept.join(delimiter))
}

/// JavaScript's `String.prototype.split`: an empty separator splits into
/// characters rather than yielding one empty field.
fn split_js<'a>(input: &'a str, delimiter: &str) -> Vec<&'a str> {
    if delimiter.is_empty() {
        let mut parts = Vec::new();
        let mut start = 0;
        for (index, character) in input.char_indices().skip(1) {
            parts.push(&input[start..index]);
            start = index;
            let _ = character;
        }
        if !input.is_empty() {
            parts.push(&input[start..]);
        }
        parts
    } else {
        input.split(delimiter).collect()
    }
}

/// Prefixes each line with its number, right-aligned to the widest number.
///
/// The width comes from the line *count*, not from the largest number
/// emitted, so a non-zero offset can overflow the column exactly as the
/// reference lets it.
pub(super) fn add_line_numbers(
    input: &str,
    offset: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let lines: Vec<&str> = input.split('\n').collect();
    let width = lines.len().to_string().len();
    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let number = i128::try_from(index + 1).unwrap_or(i128::MAX) + offset;
        let rendered = number.to_string();
        for _ in rendered.chars().count()..width {
            output.push(' ');
        }
        output.push_str(&rendered);
        output.push(' ');
        output.push_str(line);
        output.push('\n');
    }
    output.pop();
    context.ensure_active()?;
    Ok(output)
}

/// Strips a leading line number where one is trivially detectable.
///
/// Mirrors `/^[ \t]{0,5}\d+[\s:|\-,.)\]]/gm`, including the ECMAScript
/// multiline rule that `^` also matches after CR, LS, and PS — not only LF.
pub(super) fn remove_line_numbers(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let characters: Vec<char> = input.chars().collect();
    let mut output = String::new();
    let mut index = 0;
    let mut at_line_start = true;
    while index < characters.len() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if at_line_start && let Some(end) = line_number_length(&characters[index..]) {
            index += end;
            at_line_start = matches!(characters.get(index.wrapping_sub(1)), Some(character) if is_line_terminator(*character));
            continue;
        }
        let character = characters[index];
        output.push(character);
        at_line_start = is_line_terminator(character);
        index += 1;
    }
    context.ensure_active()?;
    Ok(output)
}

const fn is_line_terminator(value: char) -> bool {
    matches!(value, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Length of a `[ \t]{0,5}\d+[\s:|\-,.)\]]` match, if one starts here.
fn line_number_length(characters: &[char]) -> Option<usize> {
    let mut index = 0;
    while index < 5 && matches!(characters.get(index), Some(' ' | '\t')) {
        index += 1;
    }
    let digits_start = index;
    while matches!(characters.get(index), Some(character) if character.is_ascii_digit()) {
        index += 1;
    }
    if index == digits_start {
        return None;
    }
    let terminator = *characters.get(index)?;
    let closes = is_js_whitespace(terminator)
        || matches!(terminator, ':' | '|' | '-' | ',' | '.' | ')' | ']');
    closes.then_some(index + 1)
}

/// Pads every line to `length` extra characters with a repeating filler.
pub(super) fn pad_lines(
    input: &str,
    position: &str,
    length: i128,
    filler: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let at_start = match position {
        "Start" => true,
        "End" => false,
        _ => return Err(crate::failure::failed(INVALID_POSITION)),
    };
    let filler: Vec<u16> = filler.encode_utf16().collect();
    let mut output = String::new();
    for (index, line) in input.split('\n').enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if index > 0 {
            output.push('\n');
        }
        let padding = padding_for(&filler, length);
        if at_start {
            output.push_str(&padding);
            output.push_str(line);
        } else {
            output.push_str(line);
            output.push_str(&padding);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// JavaScript `padStart`/`padEnd` repeats the filler and truncates it to fit,
/// and produces nothing at all when the filler is empty.
///
/// The truncation is by UTF-16 code unit, not by character, so a filler made
/// of astral characters can be cut mid-pair. The reference then encodes the
/// orphaned half as U+FFFD on the way out, which is what the lossy decode
/// below reproduces.
fn padding_for(filler: &[u16], length: i128) -> String {
    let Ok(length) = usize::try_from(length) else {
        return String::new();
    };
    if filler.is_empty() {
        return String::new();
    }
    let units: Vec<u16> = filler.iter().copied().cycle().take(length).collect();
    char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}
