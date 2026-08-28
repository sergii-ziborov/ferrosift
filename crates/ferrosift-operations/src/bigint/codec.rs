use alloc::string::{String, ToString};

use ferrosift_core::{OperationContext, OperationError};
use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};

use crate::failure::failed;
use crate::jscompat::delim::is_js_whitespace;

/// Trims the exact set `String.prototype.trim` removes.
///
/// Not `str::trim`, which is the Unicode `White_Space` property and disagrees
/// with ECMAScript in both directions: JavaScript strips U+FEFF and Rust does
/// not, Rust strips U+0085 and JavaScript does not. Both operations here read
/// a number out of text the reference has already trimmed its own way, so the
/// difference decides whether a value parses at all.
pub(super) fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

/// Parses a decimal or `0x`-prefixed hexadecimal integer.
///
/// The reference accepts exactly two shapes — `/^0x[0-9a-f]+$/i` and
/// `/^[+-]?[0-9]+$/` — and refuses everything else, including a hex literal
/// carrying a sign. Both are anchored, so trailing text is an error rather
/// than being ignored the way `parseInt` would ignore it.
///
/// # Errors
///
/// Refuses text of neither shape, and refuses a literal long enough that
/// converting it would outrun the budget — `num-bigint` parses in quadratic
/// time, so a megabyte of digits is not a megabyte of work.
pub(super) fn parse_integer(
    value: &str,
    code: &'static str,
    context: &OperationContext<'_>,
) -> Result<BigInt, OperationError> {
    let trimmed = js_trim(value);
    context.ensure_work(parse_cost(trimmed.len()))?;

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

/// What converting `digits` characters into a `BigInt` is charged.
///
/// Base conversion is quadratic: each of the `digits/19` limbs is appended by
/// multiplying the accumulator through, so the limb-multiply count grows as
/// the square. The divisor turns limb multiplies into the same coarse unit the
/// rest of the budget speaks, and only the growth rate has to be right — the
/// point is that asking for a thousand times more is refused a thousand times
/// sooner, not that the number is a duration.
fn parse_cost(digits: usize) -> u64 {
    let digits = u64::try_from(digits).unwrap_or(u64::MAX);
    digits.saturating_mul(digits) / 4096
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
    let first = js_trim(first);
    let second = js_trim(second);
    let input = js_trim(input);

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

/// Chooses base and exponent, one of which may come from the input.
///
/// The same two-of-three shape as [`resolve_pair`], with one branch the older
/// operations do not have: when neither is given, the reference distinguishes
/// having nothing to work with from having an input it cannot place. An input
/// with both boxes empty could be either operand, and the reference refuses to
/// guess rather than picking one — so `missing_both` and `ambiguous` are two
/// different answers and not one message with two spellings.
pub(super) fn resolve_base_and_exponent<'a>(
    base: &'a str,
    exponent: &'a str,
    input: &'a str,
) -> Result<(&'a str, &'a str), OperationError> {
    let base = js_trim(base);
    let exponent = js_trim(exponent);
    let input = js_trim(input);

    match (base.is_empty(), exponent.is_empty()) {
        (false, false) => Ok((base, exponent)),
        (true, false) => {
            if input.is_empty() {
                return Err(failed("math.modexp.missing_base"));
            }
            Ok((input, exponent))
        }
        (false, true) => {
            if input.is_empty() {
                return Err(failed("math.modexp.missing_exponent"));
            }
            Ok((base, input))
        }
        (true, true) if input.is_empty() => Err(failed("math.modexp.missing_both")),
        (true, true) => Err(failed("math.modexp.ambiguous_input")),
    }
}

/// `base ^ exponent` modulo `modulus`, by the reference's own square-and-multiply.
///
/// Written out rather than delegated to `num-bigint`'s `modpow`, because the
/// two disagree on everything outside the textbook case and the reference's
/// answers are the ones that have to come out:
///
/// - A **negative exponent** never enters the loop, so the result is one.
///   `modpow` panics instead. Mathematically the reference is wrong here, but
///   it is what a recipe written against it produces.
/// - A **negative modulus** leaves the remainder carrying the sign of the
///   dividend, because that is what `%` does in JavaScript and in Rust alike.
///   `modpow` normalizes to a non-negative representative.
/// - A **modulus of one** with a zero exponent returns one rather than zero,
///   since the loop that would reduce it never runs.
///
/// # Errors
///
/// Refuses an exponentiation large enough to outrun the budget.
pub(super) fn modular_exponentiation(
    base: &BigInt,
    exponent: &BigInt,
    modulus: &BigInt,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_work(exponentiation_cost(exponent, modulus))?;

    let mut result = BigInt::one();
    let mut base = base % modulus;
    let mut exponent = exponent.clone();

    while exponent.is_positive() {
        if exponent.bit(0) {
            result = result * &base % modulus;
        }
        base = &base * &base % modulus;
        exponent >>= 1;
    }
    Ok(result.to_string())
}

/// What a modular exponentiation is charged.
///
/// One squaring per exponent bit, each a schoolbook multiply whose cost grows
/// as the square of the modulus's limb count. Both factors matter and neither
/// is visible from the input size, which is why this is charged rather than
/// left to the byte ceilings: a two-character recipe can name a year of work.
fn exponentiation_cost(exponent: &BigInt, modulus: &BigInt) -> u64 {
    // A non-positive exponent skips the loop entirely, so there is nothing to
    // charge for and nothing to refuse.
    if !exponent.is_positive() {
        return 0;
    }
    let rounds = exponent.bits();
    let limbs = modulus.bits().div_ceil(64).max(1);
    rounds.saturating_mul(limbs.saturating_mul(limbs)) / 64
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
