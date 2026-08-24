//! Input/output byte conversions for cipher operations.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;
use ferrosift_model::{TextEncoding, TextValue, Value};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;
use crate::key::{XOR_INVALID_KEY, convert_to_byte_array};

const INVALID_FORMAT: &str = "crypto.invalid_format";
const INVALID_HEX: &str = "crypto.invalid_hex";

/// Parses a toggleString field into raw key/IV/tag bytes.
pub(crate) fn toggle_bytes(option: &str, string: &str) -> Result<Vec<u8>, OperationError> {
    convert_to_byte_array(string, option, XOR_INVALID_KEY).map_err(|_| failed(INVALID_FORMAT))
}

/// Interprets an operation input as cipher bytes.
pub(crate) fn decode_input(input: Value, format: &str) -> Result<Vec<u8>, OperationError> {
    let raw = match input {
        Value::Bytes(bytes) => bytes,
        Value::Text(text) => text.text.into_bytes(),
        _ => return Err(OperationError::InvalidArguments),
    };
    match format {
        "Raw" | "Latin1" | "UTF8" => Ok(raw),
        "Hex" => decode_hex_digits(core::str::from_utf8(&raw).unwrap_or("")),
        "Base64" => convert_to_byte_array(
            core::str::from_utf8(&raw).map_err(|_| failed(INVALID_FORMAT))?,
            "Base64",
            INVALID_FORMAT,
        )
        .map_err(|_| failed(INVALID_FORMAT)),
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

fn decode_hex_digits(input: &str) -> Result<Vec<u8>, OperationError> {
    let mut digits = Vec::new();
    for value in input.chars() {
        if value.is_ascii_hexdigit() {
            digits.push(value);
        } else if !value.is_whitespace() {
            return Err(failed(INVALID_HEX));
        }
    }
    if !digits.len().is_multiple_of(2) {
        return Err(failed(INVALID_HEX));
    }
    let mut output = Vec::with_capacity(digits.len() / 2);
    for chunk in digits.chunks(2) {
        let text: String = chunk.iter().collect();
        output.push(u8::from_str_radix(&text, 16).map_err(|_| failed(INVALID_HEX))?);
    }
    Ok(output)
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
