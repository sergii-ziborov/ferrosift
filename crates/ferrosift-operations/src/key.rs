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
//! is a key this reads.
//!
//! The array reading does not produce bytes. `fromDecimal` is `parseInt` per
//! field with nothing after it, so a Decimal field of `300` is three hundred, a
//! field of `-` is `NaN`, and a long enough run of digits is `Infinity`;
//! `fromBinary` chunks eight characters at a time and so cannot exceed 255, but
//! a chunk starting on a non-binary character is `NaN` there too. Only two
//! operations offer either format — the bitwise family and BLAKE2 — but what
//! reaches a consumer is a JavaScript number, and
//! [`convert_to_byte_array`] hands over exactly that.
//!
//! Which coercion follows is the consumer's, and it is not one coercion:
//!
//! * **The bitwise family** never coerces the key at all. `bitOp` hands it
//!   straight to `^`, `&`, `|` or plain arithmetic and pushes the result, and
//!   only the dish reduces that to a byte on the way out.
//! * **BLAKE2, XXTEA and Scrypt** store the array into a `Uint8Array` or a
//!   `Buffer` — `ToUint8`, which [`stored_as_bytes`] is.
//!
//! Masking up front looks equivalent for the bitwise family and is not, in two
//! places. `add` and `sub` are ordinary arithmetic, so a `NaN` key propagates
//! into the output and the dish writes zero, where a key masked to zero would
//! have left the input alone. And XOR's null-preserving mode compares `o === k`
//! by identity, so a key of `300` is not a byte of `44` there — masking first
//! makes them equal and passes the byte through untouched.

use alloc::vec::Vec;

use crate::hex_util::from_hex_auto;
use crate::jscompat::delim::is_js_whitespace;
use crate::jscompat::number::{self, JsInt};
use crate::jscompat::string::str_to_byte_array;

/// The reference's Base64 alphabet, with its padding character last.
const BASE64: &[u8; 65] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";

/// `Utils.convertToByteArray`, as the numbers it really produces.
///
/// Doubles rather than bytes because the reference's array holds doubles: this
/// is `parseInt`'s output with nothing between it and the caller. A consumer
/// that wants bytes says so with [`stored_as_bytes`], which is a second step on
/// purpose — the reference has one reading and several coercions, and folding
/// them together is how a port ends up applying one consumer's rule to all of
/// them.
pub(crate) fn convert_to_byte_array(value: &str, format: &str) -> Vec<f64> {
    match fields(value, format) {
        Some(numbers) => numbers,
        // Latin1 and anything unrecognised. The reference has no error branch
        // here: its `switch` falls through to this, so a misspelt option name
        // reads the field as raw characters instead of refusing it. Every byte
        // it produces is already a byte, so nothing here can be out of range.
        None => widen(str_to_byte_array(value)),
    }
}

/// The array as the consumers that store it into a typed array receive it.
///
/// `new Uint8Array(array)`, `Buffer.from(array)`, and an element assignment
/// into a library's own `Uint8Array` are all the same conversion — `ToUint8` —
/// and between them they cover BLAKE2's key, XXTEA's key and Scrypt's salt.
pub(crate) fn stored_as_bytes(numbers: &[f64]) -> Vec<u8> {
    numbers.iter().copied().map(number::to_uint8).collect()
}

/// The failure a dish reports for a byte array holding something that is not a
/// byte.
///
/// One code for every operation that produces one, because it is one check: the
/// reference does not validate in the operation at all. `Dish.valid()` runs
/// over the finished array and the recipe stops there, so `XOR` and `SUB` fail
/// the same way for the same reason and a per-operation code would be inventing
/// a distinction the reference does not have.
pub(crate) const INVALID_BYTE_ARRAY: &str = "core.dish.invalid_byte_array";

/// Whether a number may sit in a `byteArray` dish.
///
/// `Dish.valid()` refuses an element that is `< 0` or `> 255` — and those are
/// *comparisons*, so `NaN` fails both and is waved through. That is not a
/// rounding of the rule to make it convenient: an operation whose arithmetic
/// produced `NaN` really does continue, and the value becomes a zero byte when
/// the array is finally stored. An operation whose arithmetic produced 256 does
/// not continue at all.
pub(crate) fn fits_byte_array(value: f64) -> bool {
    value.is_nan() || (0.0..=255.0).contains(&value)
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
        // `byteArrayToChars` is `String.fromCharCode`, which is `ToUint16`, and
        // every consumer of this reading then masks the code unit to a byte.
        // Two reductions modulo powers of two compose into the smaller one, so
        // the pair is `ToUint8` — the same coercion [`stored_as_bytes`] applies,
        // reached by a different route.
        Some(numbers) => stored_as_bytes(&numbers),
        None => latin1_bytes(value),
    }
}

/// The numbers the reference's array holds, or `None` for the two readings that
/// do not go through one.
///
/// UTF8 is here as bytes because both readings agree on it: one encodes to a
/// byte array and the other to a byte string with the same contents.
fn fields(value: &str, format: &str) -> Option<Vec<f64>> {
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

/// Bytes that are already bytes, as the numbers the array actually holds.
fn widen(bytes: Vec<u8>) -> Vec<f64> {
    bytes.into_iter().map(f64::from).collect()
}

/// `fromBinary(data, "Space", 8)`.
///
/// Whitespace is *removed* and the rest is then chunked eight characters at a
/// time — so the chunks run across where the spaces were rather than restarting
/// after each one. `parseInt(chunk, 2)` reads the longest binary prefix, which
/// is how a stray character ends a chunk early instead of failing it.
///
/// Eight binary digits cannot exceed 255, so this branch never produces a
/// number out of range. It can still produce `NaN`, for a chunk whose first
/// character is not a binary digit, and that survives to the consumer.
fn decode_binary(value: &str) -> Vec<f64> {
    let bits: Vec<char> = value
        .chars()
        .filter(|one| !is_js_whitespace(*one))
        .collect();
    bits.chunks(8)
        .map(|chunk| {
            let token: alloc::string::String = chunk.iter().collect();
            match number::parse(&token, 2) {
                JsInt::Nan => f64::NAN,
                JsInt::Value(parsed) => {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "eight binary digits reach 255, far inside what a double counts by ones"
                    )]
                    let widened = parsed as f64;
                    widened
                }
            }
        })
        .collect()
}

/// `fromDecimal(data, "Auto")`.
///
/// The separator is a run of anything that is neither an ASCII digit nor a
/// minus, so `1,2,3` and `1 2 3` read alike and `1-2` is one field that
/// `parseInt` reads as `1`.
///
/// This is the branch that produces numbers a byte array cannot hold. Nothing
/// bounds `parseInt` here — the reference writes the result into a plain array
/// and moves on — so `300`, `-1`, `NaN` and `Infinity` all reach the consumer
/// untouched, and what each of them means is decided there.
fn decode_decimal(value: &str) -> Vec<f64> {
    value
        .split(|one: char| !one.is_ascii_digit() && one != '-')
        .filter(|field| !field.is_empty())
        .map(number::parse_decimal)
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
