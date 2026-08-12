use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

pub(super) fn encode(
    input: &[u8],
    encode_all_special: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let capacity = input
        .len()
        .checked_mul(3)
        .ok_or(OperationError::OutputLimitExceeded)?;
    if u64::try_from(capacity).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut output = String::with_capacity(capacity);
    for (index, byte) in input.iter().copied().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        if is_safe(byte, encode_all_special) {
            output.push(char::from(byte));
        } else {
            output.push('%');
            output.push(char::from(HEX_UPPER[usize::from(byte >> 4)]));
            output.push(char::from(HEX_UPPER[usize::from(byte & 0x0f)]));
        }
    }
    context.ensure_active()?;
    Ok(output)
}

fn is_safe(byte: u8, encode_all_special: bool) -> bool {
    if byte.is_ascii_alphanumeric() {
        return true;
    }
    !encode_all_special
        && matches!(
            byte,
            b':' | b'/'
                | b'?'
                | b'#'
                | b'['
                | b']'
                | b'@'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b'%'
        )
}

/// Decodes percent-encoded text with the reference's exact fallback chain:
/// `decodeURIComponent`, and on any `URIError` the legacy `unescape`.
pub(super) fn decode(
    input: &str,
    plus_as_space: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let data: String = if plus_as_space {
        input.replace('+', "%20")
    } else {
        String::from(input)
    };
    context.ensure_active()?;
    let output = decode_uri_component(&data).unwrap_or_else(|| unescape(&data));
    context.ensure_active()?;
    Ok(output)
}

/// The ECMAScript `decodeURIComponent`: percent sequences must form strictly
/// valid UTF-8 or the whole call fails (returns `None` here).
fn decode_uri_component(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let first = hex_pair(bytes, index + 1)?;
            index += 3;
            if first < 0x80 {
                output.push(char::from(first));
                continue;
            }
            let length = match first {
                0xC0..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF7 => 4,
                _ => return None,
            };
            let mut sequence: Vec<u8> = Vec::with_capacity(length);
            sequence.push(first);
            for _ in 1..length {
                if bytes.get(index) != Some(&b'%') {
                    return None;
                }
                sequence.push(hex_pair(bytes, index + 1)?);
                index += 3;
            }
            output.push_str(core::str::from_utf8(&sequence).ok()?);
        } else {
            let character = input[index..].chars().next()?;
            output.push(character);
            index += character.len_utf8();
        }
    }
    Some(output)
}

fn hex_pair(bytes: &[u8], index: usize) -> Option<u8> {
    let high = nibble(*bytes.get(index)?)?;
    let low = nibble(*bytes.get(index + 1)?)?;
    Some((high << 4) | low)
}

fn nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

/// The Annex B `unescape`: `%uXXXX` and `%XX` become UTF-16 code units with
/// no UTF-8 interpretation, and anything malformed stays literal. Lone
/// surrogates degrade to U+FFFD exactly as the reference's UTF-8 dish
/// conversion does.
fn unescape(input: &str) -> String {
    let units: Vec<u16> = input.encode_utf16().collect();
    let mut output: Vec<u16> = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if unit == u16::from(b'%') {
            if index + 6 <= units.len()
                && units[index + 1] == u16::from(b'u')
                && let Some(value) = hex_units(&units[index + 2..index + 6])
            {
                output.push(value);
                index += 6;
                continue;
            }
            if index + 3 <= units.len()
                && let Some(value) = hex_units(&units[index + 1..index + 3])
            {
                output.push(value);
                index += 3;
                continue;
            }
        }
        output.push(unit);
        index += 1;
    }
    String::from_utf16_lossy(&output)
}

fn hex_units(units: &[u16]) -> Option<u16> {
    let mut value: u16 = 0;
    for unit in units {
        let digit = u8::try_from(*unit).ok().and_then(nibble)?;
        value = (value << 4) | u16::from(digit);
    }
    Some(value)
}
