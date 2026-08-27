//! `CyberChef`'s two readings of a toggleString key.
//!
//! There are *two*, not one, and which an operation uses is not a detail:
//! `Utils.convertToByteArray` and `Utils.convertToByteString` agree on four of
//! their six branches and part company on the two that matter most. XOR, the
//! other bitwise operations, XXTEA, BLAKE2 and Scrypt take the first; AES, AES
//! key wrapping, PBKDF2 and HMAC take the second. A port with one function is
//! wrong for half the catalog whichever behaviour it picks, and the corpus
//! family `togglestring.mjs` pins the same field read both ways to keep them
//! apart.
//!
//! Where they agree, they reach six different library functions, and this
//! reproduces each of them rather than a tidied-up reading of what they are
//! for:
//!
//! * **Hex** is permissive. It splits on anything that is not a hex digit and
//!   reads each run two characters at a time, so `abc` is two bytes and not an
//!   error, `0x41 0x42` is `AB`, and `zz` is empty.
//! * **Binary** removes whitespace and then reads the remainder eight
//!   characters at a time, across the gaps rather than resetting at them.
//! * **Decimal** splits on runs of anything that is not a digit or a minus.
//! * **Base64** removes what is not in the alphabet and decodes what is left,
//!   rather than refusing it.
//! * **UTF8** is UTF-8 in both.
//!
//! And where they differ, it is **Latin1** and *any unrecognised name*:
//! [`convert_to_byte_array`] runs `strToByteArray`, which falls back to UTF-8
//! encoding the whole string as soon as one character will not fit in a byte,
//! while [`convert_to_byte_string`] hands the string over untouched and its
//! consumer masks each code unit to eight bits. So a key of `日本` is six bytes
//! to XOR and two to HMAC.
//!
//! None of the branches can fail, which is the point: a key the reference reads
//! is a key this reads. The single exception is documented on
//! [`convert_to_byte_array`] and is a limit of the type rather than a choice.

use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;
use crate::hex_util::from_hex_auto;
use crate::jscompat::delim::is_js_whitespace;
use crate::jscompat::string::str_to_byte_array;

/// The failure code XOR has reported since it shipped; other callers pass
/// their own so a stable code always names the operation that raised it.
pub(crate) const XOR_INVALID_KEY: &str = "logic.xor.invalid_key";

/// The reference's Base64 alphabet, with its padding character last.
const BASE64: &[u8; 65] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";

/// `Utils.convertToByteArray`.
///
/// # Errors
///
/// Only for a binary or decimal field holding a number outside `0..=255`. The
/// reference builds a plain JavaScript array there, which happily holds `300`,
/// and each consumer then coerces it in its own way. A `Vec<u8>` cannot carry
/// the value through to be coerced later, so this refuses rather than picking
/// one consumer's coercion and applying it to all of them —
/// [`convert_to_byte_string`], whose consumers all mask, does the masking.
pub(crate) fn convert_to_byte_array(
    value: &str,
    format: &str,
    code: &'static str,
) -> Result<Vec<u8>, OperationError> {
    match fields(value, format) {
        Some(numbers) => numbers
            .into_iter()
            .map(|number| u8::try_from(number).map_err(|_| failed(code)))
            .collect(),
        // Latin1 and anything unrecognised. The reference has no error branch
        // here: its `switch` falls through to this, so a misspelt option name
        // reads the field as raw characters instead of refusing it.
        None => Ok(str_to_byte_array(value)),
    }
}

/// `Utils.convertToByteString`, for the operations that call that one instead.
///
/// The four decoded formats give the same bytes as [`convert_to_byte_array`] —
/// the reference turns the array into a string one character per element, and
/// the consumer masks each back down. That masking is also why this reading has
/// no error case: a field the array reading refuses as out of range survives
/// here as its low eight bits.
///
/// Latin1, and anything unrecognised, is the real difference. The reference
/// returns the string *unchanged*, so what reaches the cipher is each code unit
/// masked to a byte rather than the string's UTF-8 encoding.
/// Gated because its callers are: every operation that reads a field this way
/// is a digest or a cipher, so a build with neither pack has nothing to call it.
#[cfg(any(feature = "hash", feature = "crypto"))]
pub(crate) fn convert_to_byte_string(value: &str, format: &str) -> Vec<u8> {
    match fields(value, format) {
        Some(numbers) => numbers
            .into_iter()
            .map(|number| u8::try_from(number & 0xff).unwrap_or(0))
            .collect(),
        None => latin1_bytes(value),
    }
}

/// The numbers the reference's array holds, or `None` for the two readings that
/// do not go through one.
///
/// UTF8 is here as bytes because both readings agree on it: one encodes to a
/// byte array and the other to a byte string with the same contents.
fn fields(value: &str, format: &str) -> Option<Vec<i64>> {
    let numbers = match format.to_ascii_lowercase().as_str() {
        "binary" => decode_binary(value),
        "hex" => widen(from_hex_auto(value)),
        "decimal" => decode_decimal(value),
        "base64" => widen(decode_base64(value)),
        "utf8" => widen(value.as_bytes().to_vec()),
        _ => return None,
    };
    Some(numbers)
}

/// Bytes that are already bytes, as the wider numbers the caller compares.
fn widen(bytes: Vec<u8>) -> Vec<i64> {
    bytes.into_iter().map(i64::from).collect()
}

/// `fromBinary(data, "Space", 8)`.
///
/// Whitespace is *removed* and the rest is then chunked eight characters at a
/// time — so the chunks run across where the spaces were rather than restarting
/// after each one. `parseInt(chunk, 2)` reads the longest binary prefix, which
/// is how a stray character ends a chunk early instead of failing it.
fn decode_binary(value: &str) -> Vec<i64> {
    let bits: Vec<char> = value
        .chars()
        .filter(|one| !is_js_whitespace(*one))
        .collect();
    bits.chunks(8)
        .map(|chunk| prefix_number(chunk, 2).unwrap_or(0))
        .collect()
}

/// `fromDecimal(data, "Auto")`.
///
/// The separator is a run of anything that is neither an ASCII digit nor a
/// minus, so `1,2,3` and `1 2 3` read alike and `1-2` is one field that
/// `parseInt` reads as `1`.
fn decode_decimal(value: &str) -> Vec<i64> {
    value
        .split(|one: char| !one.is_ascii_digit() && one != '-')
        .filter(|field| !field.is_empty())
        .map(|field| {
            let digits: Vec<char> = field.chars().collect();
            // `parseInt` of a field that is only a minus sign is NaN, which
            // reads back out of a byte array as zero.
            prefix_number(&digits, 10).unwrap_or(0)
        })
        .collect()
}

/// `fromBase64(data, null, "byteArray")`, which strips rather than refuses.
///
/// Transcribed from the reference's own loop rather than written as a decoder,
/// because the loop's arithmetic is what decides the tail. A missing character
/// scores minus one, and the shifts carry that sign through, so the two derived
/// bytes fall outside `0..256` and are dropped — which is what makes an
/// unpadded `QU` one byte instead of two.
fn decode_base64(value: &str) -> Vec<u8> {
    if value.is_empty() {
        return Vec::new();
    }
    let symbols: Vec<i32> = value
        .chars()
        .filter_map(|one| {
            u8::try_from(one)
                .ok()
                .and_then(|byte| BASE64.iter().position(|entry| *entry == byte))
        })
        .filter_map(|index| i32::try_from(index).ok())
        .collect();

    let mut output = Vec::new();
    for chunk in symbols.chunks(4) {
        let at = |index: usize| chunk.get(index).copied().unwrap_or(-1);
        let (first, second, third, fourth) = (at(0), at(1), at(2), at(3));
        push_byte(&mut output, (first << 2) | (second >> 4), true);
        push_byte(
            &mut output,
            ((second & 15) << 4) | (third >> 2),
            third != 64,
        );
        push_byte(&mut output, ((third & 3) << 6) | fourth, fourth != 64);
    }
    output
}

/// Keeps a derived value when it is a byte and the padding lets it through.
fn push_byte(output: &mut Vec<u8>, value: i32, wanted: bool) {
    if wanted && (0..256).contains(&value) {
        output.push(u8::try_from(value).unwrap_or(0));
    }
}

/// The string's code units, each masked to a byte.
#[cfg(any(feature = "hash", feature = "crypto"))]
fn latin1_bytes(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .map(|unit| u8::try_from(unit & 0xff).unwrap_or(0))
        .collect()
}

/// The longest prefix of `chunk` that is a number in `radix`, as `parseInt`
/// reads it, or `None` for its `NaN`.
///
/// One optional leading minus is honoured, because a decimal field can carry
/// one: the separator the reference splits on keeps `-` out of it deliberately.
fn prefix_number(chunk: &[char], radix: u32) -> Option<i64> {
    let mut characters = chunk.iter().peekable();
    let negative = characters.next_if_eq(&&'-').is_some();
    let mut value: i64 = 0;
    let mut seen = false;
    for digit in characters {
        match digit.to_digit(radix) {
            Some(nibble) => {
                value = value
                    .saturating_mul(i64::from(radix))
                    .saturating_add(i64::from(nibble));
                seen = true;
            }
            None => break,
        }
    }
    if !seen {
        return None;
    }
    Some(if negative { -value } else { value })
}
