//! Shared lowercase hex rendering for digests and dumps.

use alloc::string::String;

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Encodes bytes as lower-case hex without separators.
pub(crate) fn to_hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
