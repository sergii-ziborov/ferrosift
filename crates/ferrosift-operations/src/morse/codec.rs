use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::jscompat::delim::char_rep;

use super::table::{SPACE, TABLE, TABLE_TAIL};

const INVALID_DELIMITER: &str = "encoding.morse.invalid_delimiter";
const INVALID_FORMAT: &str = "encoding.morse.invalid_format";

/// The delimiters the reference offers between letters.
const LETTER_DELIMITERS: [&str; 8] = [
    "Space",
    "Line feed",
    "CRLF",
    "Forward slash",
    "Backslash",
    "Comma",
    "Semi-colon",
    "Colon",
];

/// The delimiters offered between words, which deliberately exclude `Space` —
/// a space already separates letters within a word, so accepting it would make
/// the two levels indistinguishable. The reference rejects it outright, and so
/// does this.
const WORD_DELIMITERS: [&str; 7] = [
    "Line feed",
    "CRLF",
    "Forward slash",
    "Backslash",
    "Comma",
    "Semi-colon",
    "Colon",
];

/// Resolves a delimiter token, rejecting one that is not offered at this level.
fn delimiter(token: &str, allowed: &[&str]) -> Result<&'static str, OperationError> {
    if !allowed.contains(&token) {
        return Err(failed(INVALID_DELIMITER));
    }
    char_rep(token, INVALID_DELIMITER)
}

/// Every table entry in the reference's order.
fn entries() -> Vec<(char, &'static str)> {
    let mut all: Vec<(char, &'static str)> = TABLE.to_vec();
    all.extend_from_slice(&TABLE_TAIL);
    all.push(SPACE);
    all
}

/// Splits a format option such as `-/.` into its dash and dot renderings.
fn format_parts(format: &str) -> Result<(&str, &str), OperationError> {
    format.split_once('/').ok_or_else(|| failed(INVALID_FORMAT))
}

/// Encodes text as Morse.
///
/// The reference splits on line breaks, then on runs of spaces, then on
/// characters — so a character with no table entry contributes nothing while
/// still consuming a letter delimiter position, and consecutive spaces
/// collapse into one word break.
pub(super) fn encode(
    input: &str,
    format: &str,
    letter_token: &str,
    word_token: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let (dash, dot) = format_parts(format)?;
    let letter_delimiter = delimiter(letter_token, &LETTER_DELIMITERS)?;
    let word_delimiter = delimiter(word_token, &WORD_DELIMITERS)?;
    let table = entries();

    let mut lines: Vec<String> = Vec::new();
    for (index, line) in split_lines(input).iter().enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        let words: Vec<String> = split_words(line)
            .iter()
            .map(|word| {
                word.chars()
                    .map(|character| render(&table, character, dash, dot))
                    .collect::<Vec<_>>()
                    .join(letter_delimiter)
            })
            .collect();
        lines.push(words.join(word_delimiter));
    }
    context.ensure_active()?;
    Ok(lines.join("\n"))
}

/// Renders one character's signal, or nothing when the table has no entry.
fn render(table: &[(char, &str)], character: char, dash: &str, dot: &str) -> String {
    let upper = character.to_ascii_uppercase();
    let Some((_, pattern)) = table.iter().find(|(key, _)| *key == upper) else {
        return String::new();
    };
    pattern
        .chars()
        .map(|bit| if bit == '1' { dash } else { dot })
        .collect()
}

/// Splits on `\r?\n`, as the reference's line regex does.
fn split_lines(input: &str) -> Vec<&str> {
    input
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

/// Splits on runs of spaces, as the reference's ` +` regex does.
fn split_words(line: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let bytes = line.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b' ' {
            words.push(&line[start..index]);
            while index < bytes.len() && bytes[index] == b' ' {
                index += 1;
            }
            start = index;
        } else {
            index += 1;
        }
    }
    words.push(&line[start..]);
    words
}

/// The dash and dot spellings the decoder accepts before splitting.
const DASHES: [char; 6] = ['-', '\u{2010}', '\u{2212}', '_', '\u{2013}', '\u{2014}'];
const DOTS: [char; 2] = ['.', '\u{00B7}'];

/// Decodes Morse back into text.
///
/// A signal with no table entry produces nothing rather than an error, because
/// the reference joins an `undefined` lookup as an empty string.
pub(super) fn decode(
    input: &str,
    letter_token: &str,
    word_token: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let letter_delimiter = delimiter(letter_token, &LETTER_DELIMITERS)?;
    let word_delimiter = delimiter(word_token, &WORD_DELIMITERS)?;
    let table = entries();
    let normalised = normalise(input);

    let mut words: Vec<String> = Vec::new();
    for (index, word) in split_on(&normalised, word_delimiter).iter().enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        let letters: String = split_on(word, letter_delimiter)
            .iter()
            .filter_map(|signal| {
                table
                    .iter()
                    .find(|(_, pattern)| pattern == signal)
                    .map(|(character, _)| *character)
            })
            .collect();
        words.push(letters);
    }
    context.ensure_active()?;
    Ok(words.join(" "))
}

/// Rewrites every accepted dash and dot spelling into `1` and `0`.
///
/// The reference also accepts the words `dash` and `dot`, case-insensitively,
/// and replaces dashes before dots — so `dot` inside a longer word is still
/// seen after the dash pass has run.
fn normalise(input: &str) -> String {
    let lowered = input.to_lowercase();
    let mut output = String::with_capacity(lowered.len());
    let characters: Vec<char> = lowered.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        if characters[index..].starts_with(&['d', 'a', 's', 'h']) {
            output.push('1');
            index += 4;
        } else if characters[index..].starts_with(&['d', 'o', 't']) {
            output.push('0');
            index += 3;
        } else if DASHES.contains(&characters[index]) {
            output.push('1');
            index += 1;
        } else if DOTS.contains(&characters[index]) {
            output.push('0');
            index += 1;
        } else {
            output.push(characters[index]);
            index += 1;
        }
    }
    output
}

/// JavaScript's `split`, including the empty-separator case.
fn split_on(input: &str, delimiter: &str) -> Vec<String> {
    if delimiter.is_empty() {
        return input.chars().map(|value| value.to_string()).collect();
    }
    input.split(delimiter).map(ToString::to_string).collect()
}
