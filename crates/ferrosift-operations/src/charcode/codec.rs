use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use crate::delim::char_rep;
use crate::failure::failed;

const INVALID_BASE: &str = "encoding.charcode.invalid_base";
const INVALID_DELIMITER: &str = "encoding.charcode.invalid_delimiter";

pub(super) fn encode(
    input: &str,
    delimiter_token: &str,
    base: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if !(2..=36).contains(&base) {
        return Err(failed(INVALID_BASE));
    }
    let base = u32::try_from(base).map_err(|_| failed(INVALID_BASE))?;
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let mut output = String::new();
    for (index, ch) in input.chars().enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        if index > 0 {
            output.push_str(delimiter);
        }
        let code = ch as u32;
        if base == 16 {
            output.push_str(&format_hex(code));
        } else {
            output.push_str(&to_radix(code, base));
        }
    }
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    delimiter_token: &str,
    base: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if !(2..=36).contains(&base) {
        return Err(failed(INVALID_BASE));
    }
    let base = u32::try_from(base).map_err(|_| failed(INVALID_BASE))?;
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let delimiter = char_rep(delimiter_token, INVALID_DELIMITER)?;
    let mut tokens: Vec<String> = if delimiter.is_empty() {
        input.chars().map(String::from).collect()
    } else {
        input.split(delimiter).map(String::from).collect()
    };
    if tokens.len() == 1 && input.len() > 17 {
        tokens.clear();
        let mut index = 0;
        while index < input.len() {
            let end = (index + 2).min(input.len());
            tokens.push(input[index..end].to_string());
            index = end;
        }
    }
    let mut latin1 = String::new();
    for (index, token) in tokens.iter().enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        let value = u32::from_str_radix(token.trim(), base).unwrap_or(0);
        if let Some(ch) = char::from_u32(value & 0xffff) {
            // Utils.chr then strToArrayBuffer latin1: keep BMP code units truncated later.
            latin1.push(ch);
        }
    }
    let mut output = Vec::with_capacity(latin1.len());
    for unit in latin1.encode_utf16() {
        output.push(u8::try_from(unit & 0xff).unwrap_or(0));
    }
    context.ensure_active()?;
    Ok(output)
}

fn format_hex(code: u32) -> String {
    let padding = if code < 256 {
        2
    } else if code < 65_536 {
        4
    } else if code < 16_777_216 {
        6
    } else {
        8
    };
    let mut text = format!("{code:x}");
    while text.len() < padding {
        text.insert(0, '0');
    }
    text
}

const RADIX_DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

fn to_radix(mut value: u32, base: u32) -> String {
    if value == 0 {
        return String::from("0");
    }
    let mut chars = Vec::new();
    while value > 0 {
        chars.push(char::from(RADIX_DIGITS[(value % base) as usize]));
        value /= base;
    }
    chars.iter().rev().collect()
}
