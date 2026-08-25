use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const NOT_BASE92: &str = "encoding.base92.not_base92";

/// Maps a value to its Base92 symbol.
///
/// The alphabet is printable ASCII from `!` to `}` less `"` and `` ` ``, which
/// is 91 symbols — the scheme is called Base92 because the original counts a
/// 92nd character, `~`, that stands for the empty input and is never a digit.
/// Two symbols carry thirteen bits: `91 * 91 = 8281` covers `2^13 = 8192` with
/// 89 combinations left over that a well-formed encoder never emits.
fn symbol(value: u32) -> Result<u8, OperationError> {
    match value {
        0 => Ok(b'!'),
        1..=61 => Ok(b'#' + u8::try_from(value).map_err(|_| failed(NOT_BASE92))? - 1),
        62..=90 => Ok(b'a' + u8::try_from(value).map_err(|_| failed(NOT_BASE92))? - 62),
        _ => Err(failed(NOT_BASE92)),
    }
}

/// The inverse of [`symbol`], over the same three ranges.
fn ordinal(character: char) -> Result<u32, OperationError> {
    match character {
        '!' => Ok(0),
        '#'..='_' => Ok(u32::from(character) - u32::from('#') + 1),
        'a'..='}' => Ok(u32::from(character) - u32::from('a') + 62),
        _ => Err(failed(NOT_BASE92)),
    }
}

/// Encodes text as Base92.
///
/// The reference reads its input as a string and takes `charCodeAt` of each
/// character, so this walks UTF-16 code units rather than bytes. That matters
/// above U+00FF: `padStart(8, "0")` pads a short binary string but never
/// truncates a long one, so a code unit above 255 contributes more than eight
/// bits and shifts everything after it. Iterating code units reproduces that
/// exactly; iterating bytes would silently disagree on any non-Latin-1 input.
pub(super) fn encode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output: Vec<u8> = Vec::new();
    let mut bits: Vec<u8> = Vec::new();
    for (index, unit) in input.encode_utf16().enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        push_bits(&mut bits, u32::from(unit), 8);
        while bits.len() >= 13 {
            let value = take_bits(&mut bits, 13);
            output.push(symbol(value / 91)?);
            output.push(symbol(value % 91)?);
        }
    }

    // A tail shorter than seven bits is padded to six and sent as one symbol;
    // anything longer is padded to a full thirteen and sent as two. Six and
    // thirteen, not six and twelve: the short form is a genuinely different
    // encoding of the remainder rather than half of the long one.
    if !bits.is_empty() {
        if bits.len() < 7 {
            while bits.len() < 6 {
                bits.push(0);
            }
            let value = take_bits(&mut bits, 6);
            output.push(symbol(value)?);
        } else {
            while bits.len() < 13 {
                bits.push(0);
            }
            let value = take_bits(&mut bits, 13);
            output.push(symbol(value / 91)?);
            output.push(symbol(value % 91)?);
        }
    }

    context.ensure_active()?;
    String::from_utf8(output).map_err(|_| failed(NOT_BASE92))
}

/// Decodes Base92 text into bytes.
///
/// Symbols are read in pairs. A lone trailing symbol carries six bits rather
/// than thirteen, matching the encoder's short tail.
pub(super) fn decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let characters: Vec<char> = input.chars().collect();
    let mut output: Vec<u8> = Vec::new();
    let mut bits: Vec<u8> = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        if index + 1 < characters.len() {
            let value = ordinal(characters[index])? * 91 + ordinal(characters[index + 1])?;
            push_bits(&mut bits, value, 13);
        } else {
            push_bits(&mut bits, ordinal(characters[index])?, 6);
        }
        while bits.len() >= 8 {
            let byte = take_bits(&mut bits, 8);
            output.push(u8::try_from(byte).map_err(|_| failed(NOT_BASE92))?);
        }
        index += 2;
    }
    context.ensure_active()?;
    Ok(output)
}

/// Appends a value's binary digits, zero-padded to at least `minimum` places.
///
/// This is `toString(2).padStart(minimum, "0")`, and the half that matters is
/// that `padStart` widens but never truncates. A decoded symbol pair can reach
/// 8280 — `91 * 91 - 1` — which needs fourteen bits, so asking for thirteen
/// yields fourteen. Emitting exactly thirteen would drop the top bit and shift
/// every byte after it, which is a difference no ASCII sample would reveal.
fn push_bits(bits: &mut Vec<u8>, value: u32, minimum: u32) {
    let width = (u32::BITS - value.leading_zeros()).max(minimum);
    for position in (0..width).rev() {
        bits.push(u8::try_from((value >> position) & 1).unwrap_or(0));
    }
}

/// Removes the leading `count` bits and returns them as an integer.
fn take_bits(bits: &mut Vec<u8>, count: usize) -> u32 {
    let mut value = 0u32;
    for bit in bits.drain(..count) {
        value = (value << 1) | u32::from(bit);
    }
    value
}
