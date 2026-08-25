//! PEM framing around a DER body.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::base64;
use crate::failure::failed;
use crate::hex_util::{from_hex_pairs, to_hex_lower};

/// Where the reference folds the base64 body.
const FOLD_AT: usize = 64;

/// Wraps hexadecimal DER in a PEM block.
///
/// Line endings are CRLF and the block ends with one, which is the reference's
/// choice rather than the more common bare LF. Both are accepted by every
/// reader; only one is what this reference writes, and a recipe that hashes
/// the result can tell the difference.
///
/// # Errors
///
/// Returns an error if the encoded body would exceed the execution budget.
pub fn to_pem(
    input: &str,
    header: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let compact: String = input
        .chars()
        .filter(|glyph| !glyph.is_whitespace())
        .collect();
    let body = base64::encode_standard(&from_hex_pairs(&compact), context)?;

    let mut output = String::with_capacity(body.len() + header.len() * 2 + 64);
    output.push_str("-----BEGIN ");
    output.push_str(header);
    output.push_str("-----\r\n");
    output.push_str(&fold(&body));
    output.push_str("\r\n-----END ");
    output.push_str(header);
    output.push_str("-----\r\n");
    Ok(output)
}

/// Breaks the body every sixty-four characters, then trims trailing space.
///
/// The trim is what stops a body that is an exact multiple of sixty-four from
/// ending in a blank line — and it is also why an empty body leaves the block
/// with an empty line in the middle rather than none.
fn fold(body: &str) -> String {
    let mut output = String::with_capacity(body.len() + body.len() / FOLD_AT * 2);
    for (index, character) in body.chars().enumerate() {
        output.push(character);
        if (index + 1).is_multiple_of(FOLD_AT) {
            output.push_str("\r\n");
        }
    }
    String::from(output.trim_end())
}

/// Extracts every PEM block's body as hexadecimal.
///
/// More than one block is normal — a certificate chain is a file of them — so
/// every block is decoded and the results are joined with a line feed. Text
/// between and around blocks is ignored, which is what lets this read a file
/// with comments above each certificate.
///
/// # Errors
///
/// Returns an error when a block opens and never closes, or when a body is not
/// valid base64.
pub fn from_pem(input: &str, context: &OperationContext<'_>) -> Result<String, OperationError> {
    let mut blocks: Vec<String> = Vec::new();
    let mut rest = input;

    while let Some((label, after_header)) = next_header(rest) {
        let footer = alloc::format!("-----END {label}-----");
        let Some(end) = after_header.find(&footer) else {
            // A block that never closes is refused rather than read to the end
            // of the input: the bytes after it are not known to belong to it.
            return Err(failed("asn1.pem.missing_footer"));
        };
        let body = &after_header[..end];
        blocks.push(to_hex_lower(&base64::decode_standard(body, context)?));
        rest = &after_header[end + footer.len()..];
    }

    Ok(blocks.join("\n"))
}

/// Finds the next `-----BEGIN LABEL-----`, returning the label and what follows.
///
/// The label must be upper-case letters and spaces, starting and ending with a
/// letter, which is the reference's pattern. A lower-case label is not a header
/// and the scan walks past it.
fn next_header(input: &str) -> Option<(&str, &str)> {
    const OPEN: &str = "-----BEGIN ";
    const CLOSE: &str = "-----";

    let mut offset = 0;
    while let Some(at) = input[offset..].find(OPEN) {
        let start = offset + at + OPEN.len();
        if let Some(width) = input[start..].find(CLOSE) {
            let label = &input[start..start + width];
            if is_label(label) {
                return Some((label, &input[start + width + CLOSE.len()..]));
            }
        }
        offset = offset + at + OPEN.len();
    }
    None
}

/// Whether a label matches `[A-Z][A-Z ]+[A-Z]`.
fn is_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    bytes[0].is_ascii_uppercase()
        && bytes[bytes.len() - 1].is_ascii_uppercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || *byte == b' ')
}
