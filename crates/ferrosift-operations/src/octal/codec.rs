use alloc::{string::String, vec::Vec};
use core::fmt::Write;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::jscompat::delim::char_rep;
use crate::jscompat::number as jsint;

const INVALID_DELIMITER: &str = "encoding.octal.invalid_delimiter";
const VALUE_OUT_OF_RANGE: &str = "encoding.octal.value_out_of_range";

pub(super) fn encode(
    input: &[u8],
    delimiter_token: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let separator = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let capacity = input
        .len()
        .checked_mul(3 + separator.len())
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
        write!(output, "{byte:o}").map_err(|_| OperationError::OutputLimitExceeded)?;
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    delimiter_token: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let separator = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let mut output = Vec::new();
    // The reference splits on the literal delimiter and keeps empty tokens,
    // which parse to NaN and coerce to zero bytes.
    let tokens: Vec<&str> = if separator.is_empty() {
        input
            .char_indices()
            .map(|(start, value)| &input[start..start + value.len_utf8()])
            .collect()
    } else {
        input.split(separator).collect()
    };
    for (index, token) in tokens.into_iter().enumerate() {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        if output.len() as u64 >= context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
        let byte =
            jsint::to_byte(jsint::parse(token, 8)).ok_or_else(|| failed(VALUE_OUT_OF_RANGE))?;
        output.push(byte);
    }
    context.ensure_active()?;
    Ok(output)
}
