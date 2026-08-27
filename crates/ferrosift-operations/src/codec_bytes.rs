//! Input/output byte conversions for cipher operations.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;
use ferrosift_model::{TextEncoding, TextValue, Value};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;
use crate::key::convert_to_byte_string;

const INVALID_FORMAT: &str = "crypto.invalid_format";

/// Parses a toggleString field into raw key/IV/tag bytes.
///
/// The *byte string* reading, because that is the one every caller's operation
/// makes: AES, AES key wrapping and PBKDF2 all call `convertToByteString`.
/// `key.rs` says what the other reading does differently and who uses it.
pub(crate) fn toggle_bytes(option: &str, string: &str) -> Vec<u8> {
    convert_to_byte_string(string, option)
}

/// Interprets an operation input as cipher bytes.
///
/// The same reading as the key beside it, and for the same reason: these
/// operations pass their input through `convertToByteString` too, so `Raw` is
/// each code unit masked to a byte rather than the text's UTF-8 encoding, and
/// `Hex` is the permissive reading that skips whatever is not a digit.
pub(crate) fn decode_input(input: Value, format: &str) -> Result<Vec<u8>, OperationError> {
    let text = match input {
        // Bytes are already what a cipher wants; nothing is re-encoded on the
        // way in.
        Value::Bytes(bytes) if matches!(format, "Raw" | "Latin1" | "UTF8") => return Ok(bytes),
        Value::Bytes(bytes) => crate::jscompat::string::byte_array_to_utf8(&bytes),
        Value::Text(text) => text.text,
        _ => return Err(OperationError::InvalidArguments),
    };
    match format {
        "Raw" | "Latin1" | "Hex" | "Base64" | "Binary" | "Decimal" | "UTF8" => {
            Ok(convert_to_byte_string(&text, format))
        }
        _ => Err(failed(INVALID_FORMAT)),
    }
}

/// Emits cipher output as Text (hex) or Bytes (raw).
pub(crate) fn encode_output(bytes: &[u8], format: &str) -> Result<Value, OperationError> {
    match format {
        "Hex" => Ok(Value::Text(TextValue {
            text: to_hex_lower(bytes),
            encoding: TextEncoding::Utf8,
        })),
        "Raw" | "Latin1" => Ok(Value::Bytes(bytes.to_vec())),
        "UTF8" => {
            let text = core::str::from_utf8(bytes)
                .map(String::from)
                .map_err(|_| failed(INVALID_FORMAT))?;
            Ok(Value::Text(TextValue {
                text,
                encoding: TextEncoding::Utf8,
            }))
        }
        "Base64" => {
            // Minimal standard Base64 for output.
            Ok(Value::Text(TextValue {
                text: encode_base64(bytes),
                encoding: TextEncoding::Utf8,
            }))
        }
        _ => Err(failed(INVALID_FORMAT)),
    }
}

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let a = u32::from(chunk[0]);
        let b = u32::from(chunk.get(1).copied().unwrap_or(0));
        let c = u32::from(chunk.get(2).copied().unwrap_or(0));
        let triple = (a << 16) | (b << 8) | c;
        output.push(char::from(TABLE[((triple >> 18) & 0x3f) as usize]));
        output.push(char::from(TABLE[((triple >> 12) & 0x3f) as usize]));
        if chunk.len() > 1 {
            output.push(char::from(TABLE[((triple >> 6) & 0x3f) as usize]));
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(char::from(TABLE[(triple & 0x3f) as usize]));
        } else {
            output.push('=');
        }
    }
    output
}
