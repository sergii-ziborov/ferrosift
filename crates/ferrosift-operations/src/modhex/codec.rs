use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::delim::char_rep;
use crate::hex_util;

/// Modhex substitutes the sixteen hex digits with keyboard-layout-safe letters.
const MODHEX: [char; 16] = [
    'c', 'b', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'n', 'r', 't', 'u', 'v',
];

const INVALID_DELIMITER: &str = "encoding.modhex.invalid_delimiter";

/// Encodes bytes as modhex pairs.
pub(super) fn encode(
    input: &[u8],
    delimiter_token: &str,
    line_size: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    if input.is_empty() {
        return Ok(String::new());
    }
    let line_size = usize::try_from(line_size).unwrap_or(0);
    let mut output = String::new();
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        output.push(MODHEX[usize::from(byte >> 4)]);
        output.push(MODHEX[usize::from(byte & 0x0f)]);
        output.push_str(delimiter);
        // A line size of zero never wraps: the reference's `(i + 1) % 0` is
        // NaN, which never equals zero.
        if line_size > 0 && index + 1 != input.len() && (index + 1).is_multiple_of(line_size) {
            output.push('\n');
        }
    }
    for _ in 0..delimiter.chars().count() {
        output.pop();
    }
    context.ensure_active()?;
    Ok(output)
}

/// Decodes modhex pairs back into bytes.
///
/// Whitespace is stripped before anything else, and with the `Auto` delimiter
/// every non-modhex character separates fields. With an explicit delimiter a
/// stray character survives, and the reference then indexes its alphabet with
/// `-1` and concatenates the resulting `undefined` into the hex string — which
/// is reproduced here rather than silently dropped.
pub(super) fn decode(
    input: &str,
    delimiter_token: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let stripped: String = input
        .to_lowercase()
        .chars()
        .filter(|character| !crate::delim::is_js_whitespace(*character))
        .collect();
    let delimiter = if delimiter_token == "Auto" {
        None
    } else if delimiter_token == "None" {
        Some("")
    } else {
        Some(char_rep(delimiter_token, INVALID_DELIMITER)?)
    };

    let mut hex = String::new();
    for (index, character) in stripped.chars().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        match MODHEX.iter().position(|letter| *letter == character) {
            Some(value) => {
                hex.push(char::from_digit(u32::try_from(value).unwrap_or(0), 16).unwrap_or('0'));
            }
            // `Auto` treats every non-modhex character as a separator.
            None if delimiter.is_none() => {}
            None if delimiter.is_some_and(|value| value.contains(character)) => {}
            None => hex.push_str("undefined"),
        }
    }
    context.ensure_active()?;
    Ok(hex_util::from_hex_pairs(&hex))
}
