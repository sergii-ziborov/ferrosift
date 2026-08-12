use alloc::{vec, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_SCHEME: &str = "logic.xor.invalid_scheme";

pub(crate) fn apply(
    input: &[u8],
    key: &[u8],
    scheme: &str,
    null_preserving: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut key = if key.is_empty() {
        vec![0]
    } else {
        key.to_vec()
    };
    let mut output = Vec::with_capacity(input.len());
    for (index, &operand) in input.iter().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        let key_index = index % key.len();
        let mut k = key[key_index];
        if scheme == "Cascade" {
            k = input.get(index + 1).copied().unwrap_or(0);
        } else if scheme != "Standard"
            && scheme != "Input differential"
            && scheme != "Output differential"
        {
            return Err(failed(INVALID_SCHEME));
        }
        let result = if null_preserving && (operand == 0 || operand == k) {
            operand
        } else {
            operand ^ k
        };
        output.push(result);
        if scheme != "Standard"
            && scheme != "Cascade"
            && !(null_preserving && (operand == 0 || operand == k))
        {
            match scheme {
                "Input differential" => key[key_index] = operand,
                "Output differential" => key[key_index] = result,
                _ => {}
            }
        }
    }
    context.ensure_active()?;
    Ok(output)
}
