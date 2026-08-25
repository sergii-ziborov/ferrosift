use alloc::string::{String, ToString};

use ferrosift_core::OperationError;
use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};

use crate::failure::failed;

/// Parses a decimal or `0x`-prefixed hexadecimal integer.
///
/// The reference accepts exactly two shapes — `/^0x[0-9a-f]+$/i` and
/// `/^[+-]?[0-9]+$/` — and refuses everything else, including a hex literal
/// carrying a sign. Both are anchored, so trailing text is an error rather
/// than being ignored the way `parseInt` would ignore it.
pub(super) fn parse_integer(value: &str, code: &'static str) -> Result<BigInt, OperationError> {
    let trimmed = value.trim();

    if let Some(digits) = strip_hex_prefix(trimmed) {
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return BigInt::parse_bytes(digits.as_bytes(), 16).ok_or_else(|| failed(code));
        }
        return Err(failed(code));
    }

    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1i8, rest),
        None => (1i8, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(failed(code));
    }
    let magnitude = BigInt::parse_bytes(digits.as_bytes(), 10).ok_or_else(|| failed(code))?;
    Ok(if sign < 0 { -magnitude } else { magnitude })
}

fn strip_hex_prefix(value: &str) -> Option<&str> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
}

/// Chooses `a` and `b` from the arguments, falling back to the input.
///
/// The reference reads both from arguments when both are given, takes the
/// missing one from the input when exactly one is, and refuses when neither
/// is. Whitespace-only counts as missing, so `"   "` behaves like `""`.
pub(super) fn resolve_pair<'a>(
    first: &'a str,
    second: &'a str,
    input: &'a str,
    missing_first: &'static str,
    missing_second: &'static str,
    missing_both: &'static str,
) -> Result<(&'a str, &'a str), OperationError> {
    let first = first.trim();
    let second = second.trim();
    let input = input.trim();

    match (first.is_empty(), second.is_empty()) {
        (false, false) => Ok((first, second)),
        (true, false) => {
            if input.is_empty() {
                return Err(failed(missing_first));
            }
            Ok((input, second))
        }
        (false, true) => {
            if input.is_empty() {
                return Err(failed(missing_second));
            }
            Ok((first, input))
        }
        (true, true) => Err(failed(missing_both)),
    }
}

/// The extended Euclidean algorithm, iteratively.
///
/// Returns the divisor and the two Bézout coefficients, so that
/// `first * x + second * y` is the divisor. The reference notes it avoids
/// recursion to keep the stack bounded on cryptographic inputs; the same
/// reasoning applies here, where a deep recursion has nowhere to grow on a
/// bare-metal target.
///
/// The three running pairs are the remainder and the two coefficients, each
/// carrying its previous value — the classic `old_r, r` shape written out.
pub(super) fn egcd(first: &BigInt, second: &BigInt) -> (BigInt, BigInt, BigInt) {
    let (mut previous_remainder, mut remainder) = (first.clone(), second.clone());
    let (mut previous_x, mut current_x) = (BigInt::one(), BigInt::zero());
    let (mut previous_y, mut current_y) = (BigInt::zero(), BigInt::one());

    while !remainder.is_zero() {
        // Truncating division, which is what JavaScript's `/` on BigInt does.
        let quotient = &previous_remainder / &remainder;

        let next_remainder = &previous_remainder - &quotient * &remainder;
        previous_remainder = core::mem::replace(&mut remainder, next_remainder);

        let next_x = &previous_x - &quotient * &current_x;
        previous_x = core::mem::replace(&mut current_x, next_x);

        let next_y = &previous_y - &quotient * &current_y;
        previous_y = core::mem::replace(&mut current_y, next_y);
    }
    (previous_remainder, previous_x, previous_y)
}

/// The Extended GCD report, in the reference's own layout.
pub(super) fn extended_gcd_report(first: &BigInt, second: &BigInt) -> String {
    let (divisor, bezout_x, bezout_y) = egcd(first, second);
    // The reference reports the magnitude, so a negative gcd is folded.
    let gcd = divisor.abs();

    let mut output = String::from("gcd: ");
    output.push_str(&gcd.to_string());
    output.push_str("\n\nBezout coefficients:\nx = ");
    output.push_str(&bezout_x.to_string());
    output.push_str("\ny = ");
    output.push_str(&bezout_y.to_string());
    output.push_str("\n\n");
    output
}

/// The modular multiplicative inverse of `a` modulo `m`.
///
/// # Errors
///
/// Refuses a non-positive modulus, and refuses when the inverse does not exist
/// because the two are not coprime.
pub(super) fn modular_inverse(value: &BigInt, modulus: &BigInt) -> Result<String, OperationError> {
    if modulus <= &BigInt::zero() {
        return Err(failed("math.modinv.modulus_not_positive"));
    }
    // `((a % m) + m) % m` — a non-negative representative, since Rust and
    // JavaScript both give the remainder the sign of the dividend.
    let normalized = ((value % modulus) + modulus) % modulus;
    let (divisor, coefficient, _) = egcd(&normalized, modulus);

    if !divisor.is_one() && !(-&divisor).is_one() {
        return Err(failed("math.modinv.not_coprime"));
    }
    // A gcd of minus one flips the coefficient's sign, as the reference does.
    let candidate = if (-&divisor).is_one() {
        -coefficient
    } else {
        coefficient
    };
    let inverse = ((candidate % modulus) + modulus) % modulus;
    Ok(inverse.to_string())
}
