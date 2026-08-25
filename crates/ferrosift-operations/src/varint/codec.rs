use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// Parses a number the way the `BigInt` constructor does.
///
/// Leading and trailing whitespace is ignored, an empty string is zero, one
/// sign may follow, and `0x`, `0o` and `0b` select a radix. Anything else is a
/// syntax error, which the reference surfaces as an operation error.
///
/// The result is bounded to `u128`. The reference is arbitrary precision, so a
/// number past that bound is refused rather than wrapped — inventing a
/// different answer would be worse than declining to give one, and a
/// 39-digit `VarInt` is well outside what the format is used for.
fn parse_big(input: &str) -> Result<u128, OperationError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };

    let (radix, digits) = if let Some(hex) = strip_prefix_either(rest, "0x", "0X") {
        (16, hex)
    } else if let Some(octal) = strip_prefix_either(rest, "0o", "0O") {
        (8, octal)
    } else if let Some(binary) = strip_prefix_either(rest, "0b", "0B") {
        (2, binary)
    } else {
        (10, rest)
    };

    if digits.is_empty() {
        return Err(failed("encoding.varint.invalid_number"));
    }
    let mut value: u128 = 0;
    for character in digits.chars() {
        let digit = character
            .to_digit(radix)
            .ok_or_else(|| failed("encoding.varint.invalid_number"))?;
        value = value
            .checked_mul(u128::from(radix))
            .and_then(|scaled| scaled.checked_add(u128::from(digit)))
            .ok_or_else(|| failed("encoding.varint.out_of_range"))?;
    }
    if negative && value != 0 {
        // The reference rejects negatives explicitly rather than encoding a
        // two's-complement form.
        return Err(failed("encoding.varint.negative"));
    }
    Ok(value)
}

fn strip_prefix_either<'a>(value: &'a str, first: &str, second: &str) -> Option<&'a str> {
    value
        .strip_prefix(first)
        .or_else(|| value.strip_prefix(second))
}

/// Encodes a non-negative integer as a base-128 `VarInt`.
pub(super) fn encode(input: &str) -> Result<Vec<u8>, OperationError> {
    let mut value = parse_big(input)?;
    let mut output = Vec::new();
    while value >= 0x80 {
        // Seven bits at a time, high bit set to mean "more follows".
        output.push(u8::try_from(value & 0x7f).unwrap_or(0) | 0x80);
        value >>= 7;
    }
    output.push(u8::try_from(value).unwrap_or(0));
    Ok(output)
}

/// Decodes a base-128 `VarInt` into its decimal digits.
///
/// The reference stops at the first byte without the continuation bit and
/// ignores whatever follows, so a buffer holding several `VarInt`s decodes to
/// the first one.
pub(super) fn decode(input: &[u8]) -> Result<String, OperationError> {
    let mut value: u128 = 0;
    let mut shift = 0u32;
    for byte in input {
        let part = u128::from(byte & 0x7f);
        value = part
            .checked_shl(shift)
            .and_then(|shifted| value.checked_add(shifted))
            .ok_or_else(|| failed("encoding.varint.out_of_range"))?;
        if byte & 0x80 == 0 {
            break;
        }
        shift = shift
            .checked_add(7)
            .filter(|next| *next < 128)
            .ok_or_else(|| failed("encoding.varint.out_of_range"))?;
    }
    Ok(decimal(value))
}

fn decimal(mut value: u128) -> String {
    if value == 0 {
        return String::from("0");
    }
    let mut digits = Vec::new();
    while value > 0 {
        digits.push(b'0' + u8::try_from(value % 10).unwrap_or(0));
        value /= 10;
    }
    digits.reverse();
    digits.iter().map(|byte| char::from(*byte)).collect()
}

/// Decodes quoted-printable text into bytes.
///
/// Soft line breaks — an `=` at end of line, or at the very end of the input —
/// are removed first. After that an `=` followed by two hex digits is one
/// byte, and anything else is its own code unit.
pub(super) fn from_quoted_printable(input: &str) -> Result<Vec<u8>, OperationError> {
    // `=(?:\r?\n|$)` removed globally, including the end-of-input case.
    let mut joined = String::with_capacity(input.len());
    let characters: Vec<char> = input.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '=' {
            if index + 1 >= characters.len() {
                index += 1;
                continue;
            }
            if characters[index + 1] == '\n' {
                index += 2;
                continue;
            }
            if characters[index + 1] == '\r'
                && index + 2 < characters.len()
                && characters[index + 2] == '\n'
            {
                index += 3;
                continue;
            }
        }
        joined.push(characters[index]);
        index += 1;
    }

    let symbols: Vec<char> = joined.chars().collect();
    let mut output = Vec::with_capacity(symbols.len());
    let mut cursor = 0;
    while cursor < symbols.len() {
        if symbols[cursor] == '=' && cursor + 2 < symbols.len() {
            let high = symbols[cursor + 1];
            let low = symbols[cursor + 2];
            if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() {
                let value = high.to_digit(16).unwrap_or(0) * 16 + low.to_digit(16).unwrap_or(0);
                output.push(u8::try_from(value).unwrap_or(0));
                cursor += 3;
                continue;
            }
        }
        // `charCodeAt` gives a UTF-16 code unit, which is not a byte above
        // U+00FF. The reference then fails converting the result, so refusing
        // here says the same thing up front.
        let code = symbols[cursor] as u32;
        output
            .push(u8::try_from(code).map_err(|_| failed("encoding.quoted_printable.not_a_byte"))?);
        cursor += 1;
    }
    Ok(output)
}
