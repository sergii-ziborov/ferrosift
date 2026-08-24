//! A1Z26: letters as their positions in the alphabet.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use crate::delim::char_rep;
use crate::failure::failed;

use super::letters::{from_units, lowered_index, units};

const INVALID_DELIMITER: &str = "cipher.a1z26.invalid_delimiter";
const OUT_OF_RANGE: &str = "cipher.a1z26.out_of_range";
/// Emits the 1-based alphabet position of every letter, dropping the rest.
pub(super) fn a1z26_encode(
    input: &str,
    delimiter_token: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let mut parts: Vec<String> = Vec::new();
    for (position, unit) in units(input).iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if let Some(index) = lowered_index(*unit) {
            parts.push((index + 1).to_string());
        }
    }
    context.ensure_active()?;
    Ok(parts.join(delimiter))
}

/// Reads 1-based alphabet positions back into letters.
///
/// The range check is a JavaScript string-to-number comparison, and that is
/// load-bearing in two ways. An empty field coerces to `0`, so it fails the
/// `< 1` test and the whole input is rejected. A field that is not numeric at
/// all coerces to NaN, and both comparisons against NaN are false — so it
/// passes the guard, reaches `parseInt`, and becomes a NUL character.
/// Rejecting it instead would look tidier and would disagree.
pub(super) fn a1z26_decode(
    input: &str,
    delimiter_token: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    if input.is_empty() {
        return Ok(String::new());
    }
    let mut output: Vec<u16> = Vec::new();
    for (position, field) in split_js(input, delimiter).iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        match js_number(field) {
            Some(number) if !(1.0..=26.0).contains(&number) => return Err(failed(OUT_OF_RANGE)),
            Some(number) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "the value is bounded to 1..=26 by the guard above"
                )]
                output.push(number as u16 + 96);
            }
            // NaN: every comparison is false, so the guard lets it through and
            // `String.fromCharCode(NaN)` yields U+0000.
            None => output.push(0),
        }
    }
    context.ensure_active()?;
    Ok(from_units(&output))
}

/// JavaScript's `Number(string)` for the shapes this comparison can see.
///
/// Surrounding whitespace is ignored and an all-whitespace or empty string is
/// zero, which is why an empty field is rejected rather than skipped.
fn js_number(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    trimmed.parse::<f64>().ok()
}

/// JavaScript's `String.prototype.split`: an empty separator splits into
/// characters rather than yielding one field.
fn split_js<'a>(input: &'a str, delimiter: &str) -> Vec<&'a str> {
    if delimiter.is_empty() {
        let mut parts = Vec::new();
        let mut start = 0;
        for (index, _) in input.char_indices().skip(1) {
            parts.push(&input[start..index]);
            start = index;
        }
        if !input.is_empty() {
            parts.push(&input[start..]);
        }
        return parts;
    }
    input.split(delimiter).collect()
}
