//! Strip insignificant JSON whitespace without parsing.

use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};

/// Removes whitespace outside JSON strings. String contents are preserved.
pub(super) fn minify(
    input: &str,
    context: &mut OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escape = false;
    for ch in input.chars() {
        if in_string {
            output.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }
        if ch.is_whitespace() {
            continue;
        }
        output.push(ch);
    }
    Ok(output)
}
