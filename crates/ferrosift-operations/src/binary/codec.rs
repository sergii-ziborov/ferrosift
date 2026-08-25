use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::jscompat::delim::{char_rep, is_js_whitespace};
use crate::jscompat::number as jsint;

pub(super) const INVALID_DELIMITER: &str = "encoding.binary.invalid_delimiter";
pub(super) const INVALID_BYTE_LENGTH: &str = "encoding.binary.invalid_byte_length";
const VALUE_OUT_OF_RANGE: &str = "encoding.binary.value_out_of_range";

pub(super) fn encode(
    input: &[u8],
    delimiter_token: &str,
    width: usize,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let separator = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let capacity = input
        .len()
        .checked_mul(width.max(8) + separator.len())
        .ok_or(OperationError::OutputLimitExceeded)?;
    if u64::try_from(capacity).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut output = String::with_capacity(capacity);
    for (index, byte) in input.iter().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        if index > 0 {
            output.push_str(separator);
        }
        // Zero-padding to the requested width never truncates: a value that
        // needs more digits keeps them, exactly like `padStart`.
        let bits = [
            byte >> 7 & 1,
            byte >> 6 & 1,
            byte >> 5 & 1,
            byte >> 4 & 1,
            byte >> 3 & 1,
            byte >> 2 & 1,
            byte >> 1 & 1,
            byte & 1,
        ];
        let leading = bits.iter().take_while(|bit| **bit == 0).count().min(7);
        for _ in (8 - leading)..width {
            output.push('0');
        }
        for bit in &bits[leading..] {
            output.push(if *bit == 1 { '1' } else { '0' });
        }
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    delimiter_token: &str,
    byte_length: usize,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let stripped = strip_delimiter(input, delimiter_token)?;
    let mut output = Vec::new();
    let mut buffer = String::new();
    for (index, group) in stripped.chunks(byte_length).enumerate() {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        if output.len() as u64 >= context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
        buffer.clear();
        buffer.extend(group.iter());
        let byte =
            jsint::to_byte(jsint::parse(&buffer, 2)).ok_or_else(|| failed(VALUE_OUT_OF_RANGE))?;
        output.push(byte);
    }
    context.ensure_active()?;
    Ok(output)
}

/// Removes the selected delimiter the way the reference's `regexRep` table
/// does: `Space` and `None` strip every JavaScript-whitespace character,
/// `CRLF` strips exact pairs, and the rest strip one literal character.
fn strip_delimiter(input: &str, token: &str) -> Result<Vec<char>, OperationError> {
    let mut output: Vec<char> = Vec::with_capacity(input.len());
    match token {
        "Space" | "None" => {
            output.extend(input.chars().filter(|value| !is_js_whitespace(*value)));
        }
        "CRLF" => {
            let mut characters = input.chars().peekable();
            while let Some(value) = characters.next() {
                if value == '\r' && characters.peek() == Some(&'\n') {
                    characters.next();
                    continue;
                }
                output.push(value);
            }
        }
        "Comma" | "Semi-colon" | "Colon" | "Line feed" => {
            let separator = char_rep(token, INVALID_DELIMITER)?
                .chars()
                .next()
                .ok_or_else(|| failed(INVALID_DELIMITER))?;
            output.extend(input.chars().filter(|value| *value != separator));
        }
        _ => return Err(failed(INVALID_DELIMITER)),
    }
    Ok(output)
}
