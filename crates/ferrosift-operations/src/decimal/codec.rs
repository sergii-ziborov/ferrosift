use alloc::{string::String, vec::Vec};
use core::fmt::Write;

use ferrosift_core::{OperationContext, OperationError};

use crate::delim::{char_rep, is_js_whitespace};
use crate::failure::failed;
use crate::jsint::{self, JsInt};

const INVALID_DELIMITER: &str = "encoding.decimal.invalid_delimiter";
const VALUE_OUT_OF_RANGE: &str = "encoding.decimal.value_out_of_range";

pub(super) fn encode(
    input: &[u8],
    delimiter_token: &str,
    signed: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let separator = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let capacity = input
        .len()
        .checked_mul(4 + separator.len())
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
        let result = if signed {
            write!(output, "{}", byte.cast_signed())
        } else {
            write!(output, "{byte}")
        };
        result.map_err(|_| OperationError::OutputLimitExceeded)?;
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    delimiter_token: &str,
    signed: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut output = Vec::new();
    for (index, token) in split_tokens(input, delimiter_token)?
        .into_iter()
        .enumerate()
    {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        if output.len() as u64 >= context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
        let mut parsed = jsint::parse(token, 10);
        if signed
            && let JsInt::Value(value) = parsed
            && value < 0
        {
            // Negative values shift into the byte range once; anything still
            // negative afterwards is out of range, as in the reference.
            parsed = JsInt::Value(value + 256);
        }
        let byte = jsint::to_byte(parsed).ok_or_else(|| failed(VALUE_OUT_OF_RANGE))?;
        output.push(byte);
    }
    context.ensure_active()?;
    Ok(output)
}

/// Splits the input by the selected delimiter and drops empty tokens, which
/// matches splitting on the reference's delimiter regular expressions.
fn split_tokens<'a>(input: &'a str, token: &str) -> Result<Vec<&'a str>, OperationError> {
    if token == "CRLF" {
        return Ok(input
            .split("\r\n")
            .filter(|token| !token.is_empty())
            .collect());
    }
    let splitter: fn(char) -> bool = match token {
        "Auto" => |value: char| !value.is_ascii_digit() && value != '-',
        "Space" | "None" => is_js_whitespace,
        "Comma" => |value| value == ',',
        "Semi-colon" => |value| value == ';',
        "Colon" => |value| value == ':',
        "Line feed" => |value| value == '\n',
        _ => return Err(failed(INVALID_DELIMITER)),
    };
    Ok(input
        .split(splitter)
        .filter(|token| !token.is_empty())
        .collect())
}
