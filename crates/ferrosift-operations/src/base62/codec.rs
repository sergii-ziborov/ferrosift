//! Base62 over an arbitrary-precision integer.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

use crate::alphabet::expand;
use crate::failure::failed;

/// The base. Fixed: the reference names the operation after it.
const BASE: usize = 62;

/// Expands and validates an alphabet the way the reference's bignum does.
///
/// Two separate rejections, in the reference's order. The library validates the
/// alphabet when it is installed — at least two characters, no duplicates, and
/// none of `+`, `-`, `.` or whitespace, because those would collide with sign
/// and decimal-point syntax. Only afterwards does use check that it is long
/// enough for the base, so a short but well-formed alphabet fails at a
/// different point than a malformed one, for a different reason.
///
/// Characters past the sixty-second are kept rather than rejected. They are not
/// digits, but they are still alphabet members, which matters on the way in:
/// see [`decode`].
///
/// # Errors
///
/// Returns an error when the alphabet is malformed or shorter than the base.
pub fn resolve(expression: &str) -> Result<Vec<char>, OperationError> {
    let expanded = expand(expression, "encoding.base62.invalid_alphabet")?;

    let malformed = expanded.len() < 2
        || expanded
            .iter()
            .any(|glyph| matches!(glyph, '+' | '-' | '.') || glyph.is_whitespace())
        || (1..expanded.len()).any(|at| expanded[..at].contains(&expanded[at]));
    if malformed {
        return Err(failed("encoding.base62.malformed_alphabet"));
    }
    if expanded.len() < BASE {
        return Err(failed("encoding.base62.alphabet_too_short"));
    }
    Ok(expanded)
}

/// Renders bytes as a base-62 number.
///
/// The bytes are read as one big-endian integer, so leading zero bytes carry no
/// value and do not survive the round trip. Worth stating because Base58 sits
/// next to this in the catalog and *does* preserve them: the difference belongs
/// to the two reference implementations, not to the two bases.
#[must_use]
pub fn encode(input: &[u8], alphabet: &[char]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut value = BigUint::from_bytes_be(input);
    if value.is_zero() {
        return alphabet[0].into();
    }

    let base = BigUint::from(BASE);
    let mut digits: Vec<char> = Vec::new();
    while !value.is_zero() {
        let index = (&value % &base).to_usize().unwrap_or_default();
        value /= &base;
        digits.push(alphabet[index]);
    }
    digits.reverse();
    digits.into_iter().collect()
}

/// Reads a base-62 number back into bytes.
///
/// Characters outside the alphabet are dropped rather than refused, which makes
/// the operation tolerant of formatting. An input left empty by that filter is
/// read as zero rather than as an error, so `"!"` and `"0"` both decode to a
/// single zero byte under the default alphabet.
///
/// An alphabet longer than the base makes those two rules collide, and the
/// reference resolves the collision by failing: a character past the
/// sixty-second survives the filter, because it is in the alphabet, but is not
/// a base-62 digit, so the number is refused rather than the character being
/// quietly skipped. Skipping would be the friendlier reading and the wrong one.
///
/// # Errors
///
/// Returns an error when a character survives the filter but is not a digit.
pub fn decode(input: &str, alphabet: &[char]) -> Result<Vec<u8>, OperationError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let base = BigUint::from(BASE);
    let mut value = BigUint::ZERO;
    for glyph in input.chars() {
        let Some(index) = alphabet.iter().position(|candidate| *candidate == glyph) else {
            continue;
        };
        if index >= BASE {
            return Err(failed("encoding.base62.not_a_digit"));
        }
        value = value * &base + BigUint::from(index);
    }

    // The reference renders the number as hex, left-pads it to a whole number
    // of bytes, and converts. That is the minimal big-endian encoding, which is
    // what `to_bytes_be` produces — including the single zero byte for zero.
    Ok(value.to_bytes_be())
}
