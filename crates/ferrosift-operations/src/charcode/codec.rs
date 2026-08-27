use alloc::{format, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::jscompat::delim::char_rep;

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
    // `bites.length === 1 && input.length > 17` there, then `input.slice(i,
    // i + 2)` in a loop. Both are over a JavaScript string, so both count
    // *UTF-16 code units* -- and this counted UTF-8 bytes, which is the same
    // number only while the input is ASCII. It was wrong for every other input
    // and it panicked for the ones where a pair landed inside a character:
    // `"ˉ"` is two bytes, and slicing a `str` between them is not a smaller
    // answer but an abort. Found by `fuzz/fuzz_targets/decoders.rs`.
    let units: Vec<u16> = input.encode_utf16().collect();
    if tokens.len() == 1 && units.len() > 17 {
        tokens.clear();
        for pair in units.chunks(2) {
            // A lone surrogate becomes the replacement character, which is not
            // a digit in any base -- and neither was the surrogate, so both
            // read as zero exactly as the reference's `parseInt` does.
            tokens.push(String::from_utf16_lossy(pair));
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
