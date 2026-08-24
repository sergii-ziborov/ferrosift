use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

/// The six characters the reference can strip, in argument order.
///
/// It replaces these literals rather than matching a whitespace class, so this
/// is the complete set however the input is encoded.
pub(super) const STRIPPABLE: [char; 6] = [' ', '\r', '\n', '\t', '\u{000C}', '.'];

/// Removes every character in `selection`.
pub(super) fn remove_whitespace(
    input: &str,
    selection: &[char],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::with_capacity(input.len());
    for (index, character) in input.chars().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if !selection.contains(&character) {
            output.push(character);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// Removes every `0x00` byte.
pub(super) fn remove_null_bytes(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let mut output = Vec::with_capacity(input.len());
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if *byte != 0 {
            output.push(*byte);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// Whether `take_or_drop_nth` keeps or discards the selected bytes.
#[derive(Clone, Copy)]
pub(super) enum Nth {
    Take,
    Drop,
}

const INVALID_EVERY: &str = "data.nth_bytes.invalid_every";
const INVALID_START: &str = "data.nth_bytes.invalid_start";

/// Keeps or drops every nth byte, counting from a starting offset.
///
/// With `each_line`, the offset restarts after every `0x0a`, which is always
/// preserved regardless of the selection.
pub(super) fn take_or_drop_nth(
    input: &[u8],
    mode: Nth,
    every: i128,
    start: i128,
    each_line: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if every <= 0 {
        return Err(crate::failure::failed(INVALID_EVERY));
    }
    if start < 0 {
        return Err(crate::failure::failed(INVALID_START));
    }
    let mut output = Vec::new();
    let mut offset: i128 = 0;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let position = i128::try_from(index).unwrap_or(i128::MAX);
        if each_line && *byte == 0x0a {
            output.push(0x0a);
            offset = position + 1;
            continue;
        }
        let selected =
            position - offset >= start && (position - (start + offset)).rem_euclid(every) == 0;
        let keep = match mode {
            Nth::Take => selected,
            Nth::Drop => !selected,
        };
        if keep {
            output.push(*byte);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

const INVALID_SCOPE: &str = "data.reverse.invalid_scope";

/// Reverses the input by byte, by character, or by line.
pub(super) fn reverse(
    input: &[u8],
    scope: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    match scope {
        "Byte" => {
            let mut output = input.to_vec();
            output.reverse();
            Ok(output)
        }
        "Character" => Ok(reverse_characters(input)),
        "Line" => Ok(reverse_lines(input, context)?),
        _ => Err(crate::failure::failed(INVALID_SCOPE)),
    }
}

/// Reverses UTF-16 code units while keeping surrogate pairs intact, which is
/// what the reference's index-walking loop achieves.
fn reverse_characters(input: &[u8]) -> Vec<u8> {
    let decoded = String::from_utf8_lossy(input);
    let mut reversed: String = String::with_capacity(decoded.len());
    for character in decoded.chars().rev() {
        reversed.push(character);
    }
    reversed.into_bytes()
}

/// Reverses line order, then truncates back to the original length so a input
/// without a trailing newline does not gain one.
fn reverse_lines(input: &[u8], context: &OperationContext<'_>) -> Result<Vec<u8>, OperationError> {
    let mut lines: Vec<&[u8]> = Vec::new();
    let mut start = 0;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if *byte == 0x0a {
            lines.push(&input[start..index]);
            start = index + 1;
        }
    }
    lines.push(&input[start..]);
    lines.reverse();
    let mut output = Vec::with_capacity(input.len() + 1);
    for line in lines {
        output.extend_from_slice(line);
        output.push(0x0a);
    }
    output.truncate(input.len());
    Ok(output)
}
