use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// Expands a `CyberChef` alphabet range expression such as `A-Za-z0-9+/=`.
///
/// `a-b` spans become inclusive character ranges, and `\-` escapes a literal
/// hyphen. Invalid range endpoints report the caller's stable failure code.
pub(crate) fn expand(expression: &str, code: &'static str) -> Result<Vec<char>, OperationError> {
    let input: Vec<_> = expression.chars().collect();
    let mut output = Vec::new();
    let mut index = 0;
    while index < input.len() {
        if index + 2 < input.len() && input[index + 1] == '-' && input[index] != '\\' {
            let start = u32::from(input[index]);
            let end = u32::from(input[index + 2]);
            for value in start..=end {
                output.push(char::from_u32(value).ok_or_else(|| failed(code))?);
            }
            index += 3;
        } else if index + 1 < input.len() && input[index] == '\\' && input[index + 1] == '-' {
            output.push('-');
            index += 2;
        } else {
            output.push(input[index]);
            index += 1;
        }
    }
    Ok(output)
}
