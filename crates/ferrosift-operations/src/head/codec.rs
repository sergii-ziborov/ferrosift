use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::jscompat::delim::char_rep;

const INVALID_DELIMITER: &str = "data.head.invalid_delimiter";

pub(super) fn head(
    input: &str,
    delimiter_token: &str,
    number: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let parts: Vec<String> = if delimiter.is_empty() {
        // JavaScript `split("")` yields BMP code units; chars match for our fixtures.
        input.chars().map(String::from).collect()
    } else {
        input.split(delimiter).map(String::from).collect()
    };
    let total = i128::try_from(parts.len()).unwrap_or(i128::MAX);
    let mut kept = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        let line_index = i128::try_from(index + 1).unwrap_or(i128::MAX);
        let keep = if number < 0 {
            line_index <= total + number
        } else {
            line_index <= number
        };
        if keep {
            kept.push(part.as_str());
        }
    }
    context.ensure_active()?;
    Ok(kept.join(delimiter))
}
