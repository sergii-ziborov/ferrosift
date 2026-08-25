use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// The three escape prefixes the reference offers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Prefix {
    Backslash,
    Percent,
    UPlus,
}

pub(super) fn prefix(value: &str) -> Result<Prefix, OperationError> {
    match value {
        "\\u" => Ok(Prefix::Backslash),
        "%u" => Ok(Prefix::Percent),
        "U+" => Ok(Prefix::UPlus),
        _ => Err(failed("text.unicode.invalid_prefix")),
    }
}

impl Prefix {
    const fn text(self) -> &'static str {
        match self {
            Self::Backslash => "\\u",
            Self::Percent => "%u",
            Self::UPlus => "U+",
        }
    }

    /// How many hex digits the decoder accepts.
    ///
    /// The reference builds `{4}` for the first two prefixes and `{4,6}` for
    /// `U+`, so only that one can carry an astral code point in a single
    /// escape.
    const fn max_digits(self) -> usize {
        match self {
            Self::UPlus => 6,
            _ => 4,
        }
    }
}

/// Escapes characters as `prefix` plus padded hex.
///
/// The reference indexes the input with `[i]`, which walks UTF-16 code units,
/// and asks each one for `codePointAt(0)`. For a character outside the basic
/// plane that yields the surrogate halves separately, so an emoji becomes two
/// escapes rather than one. Iterating code units here reproduces that; walking
/// Rust characters would silently produce a different answer for exactly the
/// input a caller reaches for this operation to inspect.
pub(super) fn escape(
    input: &str,
    prefix: Prefix,
    encode_all: bool,
    padding: usize,
    upper: bool,
) -> String {
    let mut output = String::with_capacity(input.len());
    for unit in input.encode_utf16() {
        // The reference whitelists printable ASCII with `/[ -~]/`.
        if !encode_all
            && (0x20..=0x7e).contains(&unit)
            && let Some(value) = char::from_u32(u32::from(unit))
        {
            output.push(value);
            continue;
        }
        output.push_str(prefix.text());
        let digits = if upper {
            hex_upper(u32::from(unit))
        } else {
            hex_lower(u32::from(unit))
        };
        // `padStart` only pads; a value longer than the padding is untouched.
        for _ in digits.len()..padding {
            output.push('0');
        }
        output.push_str(&digits);
    }
    output
}

fn hex_lower(value: u32) -> String {
    hex_with(value, b"0123456789abcdef")
}

fn hex_upper(value: u32) -> String {
    hex_with(value, b"0123456789ABCDEF")
}

/// Hex without a leading zero, as `Number.prototype.toString(16)` produces.
fn hex_with(value: u32, digits: &[u8; 16]) -> String {
    if value == 0 {
        return String::from("0");
    }
    let mut reversed = Vec::new();
    let mut remaining = value;
    while remaining > 0 {
        reversed.push(digits[(remaining % 16) as usize]);
        remaining /= 16;
    }
    reversed.reverse();
    reversed.iter().map(|byte| char::from(*byte)).collect()
}

/// Replaces escapes with the characters they name.
///
/// Anything that is not a complete escape is copied through, which is what the
/// reference's scan does with the text between matches. Decoding goes through
/// UTF-16 so that an adjacent pair of surrogate escapes — the shape the
/// encoder above produces for an astral character — recombines into the one
/// character it came from.
pub(super) fn unescape(input: &str, prefix: Prefix) -> String {
    let characters: Vec<char> = input.chars().collect();
    let marker: Vec<char> = prefix.text().chars().collect();
    let max_digits = prefix.max_digits();

    let mut units: Vec<u16> = Vec::with_capacity(characters.len());
    let mut index = 0;
    while index < characters.len() {
        if !characters[index..].starts_with(&marker) {
            let mut buffer = [0u16; 2];
            units.extend_from_slice(characters[index].encode_utf16(&mut buffer));
            index += 1;
            continue;
        }

        // Greedy, like the regex: take as many hex digits as the quantifier
        // allows, then require at least four.
        let start = index + marker.len();
        let mut cursor = start;
        while cursor < characters.len()
            && cursor - start < max_digits
            && characters[cursor].is_ascii_hexdigit()
        {
            cursor += 1;
        }
        if cursor - start < 4 {
            let mut buffer = [0u16; 2];
            units.extend_from_slice(characters[index].encode_utf16(&mut buffer));
            index += 1;
            continue;
        }

        let mut value: u32 = 0;
        for digit in &characters[start..cursor] {
            value = value * 16 + digit.to_digit(16).unwrap_or(0);
        }
        push_code_point(&mut units, value);
        index = cursor;
    }

    char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Appends a code point as the reference's `Utils.chr` would.
///
/// Above the basic plane it splits into a surrogate pair by hand, which is the
/// same pair `encode_utf16` would produce; below it the value is one unit,
/// surrogates included, so a pair written as two escapes rejoins on decode.
fn push_code_point(units: &mut Vec<u16>, value: u32) {
    if value > 0xffff {
        let adjusted = value - 0x1_0000;
        units.push(u16::try_from((adjusted >> 10 & 0x3ff) | 0xd800).unwrap_or(0xfffd));
        units.push(u16::try_from(0xdc00 | (adjusted & 0x3ff)).unwrap_or(0xfffd));
    } else {
        units.push(u16::try_from(value).unwrap_or(0xfffd));
    }
}

/// Encodes a `NetBIOS` name: each nibble becomes a byte offset from a base.
///
/// The reference pads the name to sixteen bytes with spaces and refuses
/// anything longer by returning nothing at all.
pub(super) fn netbios_encode(input: &[u8], offset: u8) -> Vec<u8> {
    if input.len() > 16 {
        return Vec::new();
    }
    let mut padded = [b' '; 16];
    padded[..input.len()].copy_from_slice(input);

    let mut output = Vec::with_capacity(32);
    for byte in padded {
        output.push((byte >> 4).wrapping_add(offset));
        output.push((byte & 0x0f).wrapping_add(offset));
    }
    output
}

/// Decodes a `NetBIOS` name.
///
/// The trailing-space trim is reproduced exactly as written rather than as
/// intended. The reference walks backwards calling `splice(i, i)`, which
/// removes `i` elements starting at `i` rather than the single element it
/// looks like it means to remove, and stops at the first byte that is not a
/// space. For the usual sixteen-byte padded name the two agree; for other
/// inputs they do not, and the reference's answer is the one being matched.
pub(super) fn netbios_decode(input: &[u8], offset: u8) -> Result<Vec<u8>, OperationError> {
    if input.len() > 32 || !input.len().is_multiple_of(2) {
        return Ok(Vec::new());
    }
    let mut output: Vec<u8> = Vec::with_capacity(input.len() / 2);
    for pair in input.chunks_exact(2) {
        // The reference masks the low nibble but not the high one, so a first
        // byte more than fifteen above the offset shifts past a byte and it
        // produces a number no longer in range. It then fails converting that
        // to a byte array, which is to say it declines to answer. Declining
        // here is the same behaviour stated as a rejection rather than
        // discovered downstream — and it is the reason this operation cannot
        // silently wrap where the reference stops.
        let high = pair[0]
            .checked_sub(offset)
            .filter(|value| *value <= 0x0f)
            .ok_or_else(|| failed("encoding.netbios.out_of_range"))?;
        let low = pair[1].wrapping_sub(offset);
        output.push((high << 4) | (low & 0x0f));
    }

    let mut index = output.len().saturating_sub(1);
    while index > 0 {
        if output[index] != b' ' {
            break;
        }
        // `splice(i, i)` removes `i` elements from position `i`, which is what
        // runs here — not a single-element removal.
        let end = index.saturating_add(index).min(output.len());
        output.drain(index..end);
        index -= 1;
    }
    Ok(output)
}
