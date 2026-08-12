//! `CyberChef` `Utils.convertToByteArray` for toggleString keys.

use alloc::{string::String, vec::Vec};

use ferrosift_core::OperationError;

use crate::failure::failed;
use crate::jsint::{self, JsInt};

const INVALID_KEY: &str = "logic.xor.invalid_key";

/// Converts a `CyberChef` toggleString key into raw bytes.
pub(crate) fn convert_to_byte_array(value: &str, format: &str) -> Result<Vec<u8>, OperationError> {
    match format.to_ascii_lowercase().as_str() {
        "binary" => decode_binary(value),
        "hex" => decode_hex(value),
        "decimal" => decode_decimal(value),
        "base64" => decode_base64(value),
        "utf8" => Ok(value.as_bytes().to_vec()),
        "latin1" => Ok(latin1_bytes(value)),
        _ => Err(failed(INVALID_KEY)),
    }
}

fn latin1_bytes(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .map(|unit| u8::try_from(unit & 0xff).unwrap_or(0))
        .collect()
}

fn decode_hex(input: &str) -> Result<Vec<u8>, OperationError> {
    let mut digits = Vec::new();
    for value in input.chars() {
        if value.is_ascii_hexdigit() {
            digits.push(value);
        } else if value.is_ascii_alphanumeric() {
            return Err(failed(INVALID_KEY));
        }
    }
    if digits.len() % 2 == 1 {
        return Err(failed(INVALID_KEY));
    }
    let mut output = Vec::with_capacity(digits.len() / 2);
    for chunk in digits.chunks(2) {
        let text: String = chunk.iter().collect();
        let byte = u8::from_str_radix(&text, 16).map_err(|_| failed(INVALID_KEY))?;
        output.push(byte);
    }
    Ok(output)
}

fn decode_decimal(input: &str) -> Result<Vec<u8>, OperationError> {
    let mut output = Vec::new();
    for token in input.split(|value: char| !value.is_ascii_digit() && value != '-' && value != '+')
    {
        if token.is_empty() {
            continue;
        }
        match jsint::parse(token, 10) {
            JsInt::Nan => return Err(failed(INVALID_KEY)),
            JsInt::Value(value) => {
                let byte = u8::try_from(value).map_err(|_| failed(INVALID_KEY))?;
                output.push(byte);
            }
        }
    }
    Ok(output)
}

fn decode_binary(input: &str) -> Result<Vec<u8>, OperationError> {
    let mut bits = String::new();
    for value in input.chars() {
        if value == '0' || value == '1' {
            bits.push(value);
        } else if !value.is_whitespace() && value != ':' && value != ',' {
            return Err(failed(INVALID_KEY));
        }
    }
    if bits.is_empty() {
        return Ok(Vec::new());
    }
    let mut output = Vec::new();
    let mut index = 0;
    while index < bits.len() {
        let end = (index + 8).min(bits.len());
        let chunk = &bits[index..end];
        let value = u8::from_str_radix(chunk, 2).map_err(|_| failed(INVALID_KEY))?;
        let shifted = if chunk.len() < 8 {
            value << (8 - chunk.len())
        } else {
            value
        };
        output.push(shifted);
        index = end;
    }
    Ok(output)
}

fn decode_base64(input: &str) -> Result<Vec<u8>, OperationError> {
    fn value_of(symbol: u8) -> Option<u8> {
        match symbol {
            b'A'..=b'Z' => Some(symbol - b'A'),
            b'a'..=b'z' => Some(symbol - b'a' + 26),
            b'0'..=b'9' => Some(symbol - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut symbols = Vec::new();
    for value in input.chars() {
        if value == '=' || value.is_whitespace() {
            continue;
        }
        if value.is_ascii()
            && let Some(symbol) = value_of(value as u8)
        {
            symbols.push(symbol);
            continue;
        }
        return Err(failed(INVALID_KEY));
    }
    let mut output = Vec::with_capacity(symbols.len() * 3 / 4);
    for chunk in symbols.chunks(4) {
        let a = u32::from(chunk[0]);
        let b = u32::from(chunk.get(1).copied().unwrap_or(0));
        let c = u32::from(chunk.get(2).copied().unwrap_or(0));
        let d = u32::from(chunk.get(3).copied().unwrap_or(0));
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        if chunk.len() >= 2 {
            output.push(((triple >> 16) & 0xff) as u8);
        }
        if chunk.len() >= 3 {
            output.push(((triple >> 8) & 0xff) as u8);
        }
        if chunk.len() >= 4 {
            output.push((triple & 0xff) as u8);
        }
    }
    Ok(output)
}
