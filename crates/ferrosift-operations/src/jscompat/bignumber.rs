//! Arbitrary-precision arithmetic with the reference library's semantics.
//!
//! The rendering of a decimal was pinned first, and rendering is the easy
//! half: one value, one answer. Arithmetic has a configuration, and the
//! configuration is where a port goes wrong quietly.
//!
//! Three settings decide everything here, and they are the library's own
//! defaults rather than a choice made in this file:
//!
//! | Setting | Value | What it decides |
//! |---|---|---|
//! | `DECIMAL_PLACES` | 20 | how far an inexact result is carried |
//! | `ROUNDING_MODE` | 4 | half away from zero, at that last place |
//! | `MODULO_MODE` | 1 | truncated, so a remainder takes the dividend's sign |
//!
//! The asymmetry is the thing to get right. Addition, subtraction and
//! multiplication are **exact** and ignore all three: `0.1 + 0.2` is `0.3` and
//! not a rounded approximation of it. Division and square root are not exact
//! and obey all three. A port that rounded a sum, or that failed to round a
//! quotient, would pass a great many tests before failing one.

use alloc::string::ToString;
use alloc::vec::Vec;

use ferrosift_model::DecimalValue;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, Signed, Zero};

/// How far an inexact result is carried, from the library's own configuration.
const DECIMAL_PLACES: i64 = 20;

/// A value split into the integer it is and the power of ten it is scaled by.
///
/// `value * 10^exponent`. Working here rather than on the rendering matters:
/// a decimal with a large exponent renders as millions of characters, and
/// arithmetic that went through the rendering would build them first.
struct Scaled {
    value: BigInt,
    exponent: i64,
}

impl Scaled {
    /// Reads an ordinary number, or `None` when it is not one.
    fn from(decimal: &DecimalValue) -> Option<Self> {
        let (negative, digits, exponent) = decimal.parts()?;
        let magnitude = if digits.is_empty() {
            BigUint::zero()
        } else {
            BigUint::parse_bytes(digits.as_bytes(), 10)?
        };
        let sign = if negative && !magnitude.is_zero() {
            Sign::Minus
        } else {
            Sign::Plus
        };
        Some(Self {
            value: BigInt::from_biguint(sign, magnitude),
            exponent,
        })
    }

    /// Rescales to the given exponent, which must not be larger than this one.
    fn rescaled(&self, exponent: i64) -> BigInt {
        let shift = self.exponent.saturating_sub(exponent);
        if shift <= 0 {
            return self.value.clone();
        }
        self.value.clone() * BigInt::from(power_of_ten(shift.unsigned_abs()))
    }

    /// Roughly the power of ten this value sits at -- the exponent it would
    /// carry beside a single leading digit, which is the quantity the model's
    /// range applies to.
    ///
    /// Counted from the binary width rather than by rendering the coefficient:
    /// the same lesson the rendering budget already learned, that measuring a
    /// value must not cost what building it would. The estimate is good to
    /// within a digit either way, so every caller leaves a margin.
    fn scale(&self) -> i64 {
        if self.value.is_zero() {
            return 0;
        }
        // log10(2), kept as a ratio so this stays integer arithmetic.
        let bits = i64::try_from(self.value.bits()).unwrap_or(i64::MAX);
        let digits = bits.saturating_mul(30_103) / 100_000;
        self.exponent.saturating_add(digits)
    }

    /// Back to a decimal, normalised and range-checked by the model.
    fn into_decimal(self) -> DecimalValue {
        let negative = self.value.is_negative();
        let digits = self.value.magnitude().to_string();
        DecimalValue::from_parts(negative, &digits, self.exponent)
    }
}

/// Ten raised to `power`.
///
/// By repeated squaring rather than a loop of multiplications: the powers here
/// reach the model's exponent range, and ten million separate multiplications
/// would be the slowest thing in the crate by a wide margin.
fn power_of_ten(power: u64) -> BigUint {
    // Every caller bounds the power by the model's range plus the digits of
    // its operands, so the narrowing below has nothing to lose. Saturating
    // rather than unwrapping keeps the function total.
    BigUint::from(10_u32).pow(u32::try_from(power).unwrap_or(u32::MAX))
}

/// What kind of value this is, for the rules the specials follow.
enum Kind {
    NotANumber,
    Infinite { negative: bool },
    Ordinary,
}

fn kind(value: &DecimalValue) -> Kind {
    if value.is_not_a_number() {
        Kind::NotANumber
    } else if value.is_infinite() {
        Kind::Infinite {
            negative: value.is_negative(),
        }
    } else {
        Kind::Ordinary
    }
}

/// Exact addition.
#[must_use]
pub fn plus(left: &DecimalValue, right: &DecimalValue) -> DecimalValue {
    match (kind(left), kind(right)) {
        (Kind::NotANumber, _) | (_, Kind::NotANumber) => DecimalValue::not_a_number(),
        // Infinities of opposite sign have no sum, so the answer is not a
        // number rather than either of them or zero.
        (Kind::Infinite { negative: a }, Kind::Infinite { negative: b }) => {
            if a == b {
                DecimalValue::infinity(a)
            } else {
                DecimalValue::not_a_number()
            }
        }
        (Kind::Infinite { negative }, Kind::Ordinary)
        | (Kind::Ordinary, Kind::Infinite { negative }) => DecimalValue::infinity(negative),
        (Kind::Ordinary, Kind::Ordinary) => combine(left, right, false),
    }
}

/// Exact subtraction.
#[must_use]
pub fn minus(left: &DecimalValue, right: &DecimalValue) -> DecimalValue {
    match (kind(left), kind(right)) {
        (Kind::NotANumber, _) | (_, Kind::NotANumber) => DecimalValue::not_a_number(),
        (Kind::Infinite { negative: a }, Kind::Infinite { negative: b }) => {
            if a == b {
                DecimalValue::not_a_number()
            } else {
                DecimalValue::infinity(a)
            }
        }
        (Kind::Infinite { negative }, Kind::Ordinary) => DecimalValue::infinity(negative),
        (Kind::Ordinary, Kind::Infinite { negative }) => DecimalValue::infinity(!negative),
        (Kind::Ordinary, Kind::Ordinary) => combine(left, right, true),
    }
}

/// Adds or subtracts two ordinary values without losing a digit.
///
/// Both are brought to the finer of the two exponents first, which is what
/// makes the result exact: `0.1 + 0.2` is computed as `1 + 2` scaled by a
/// tenth, and answers `0.3` rather than an approximation of it.
fn combine(left: &DecimalValue, right: &DecimalValue, subtract: bool) -> DecimalValue {
    let (Some(a), Some(b)) = (Scaled::from(left), Scaled::from(right)) else {
        return DecimalValue::not_a_number();
    };
    let exponent = a.exponent.min(b.exponent);
    let first = a.rescaled(exponent);
    let second = b.rescaled(exponent);
    let value = if subtract {
        first - second
    } else {
        first + second
    };
    Scaled { value, exponent }.into_decimal()
}

/// Exact multiplication.
#[must_use]
pub fn times(left: &DecimalValue, right: &DecimalValue) -> DecimalValue {
    match (kind(left), kind(right)) {
        (Kind::NotANumber, _) | (_, Kind::NotANumber) => DecimalValue::not_a_number(),
        // Infinity times zero has no value; every other product with an
        // infinity is an infinity whose sign is the product of the two.
        (Kind::Infinite { negative }, Kind::Ordinary) => {
            if right.is_zero() {
                DecimalValue::not_a_number()
            } else {
                DecimalValue::infinity(negative != right.is_negative())
            }
        }
        (Kind::Ordinary, Kind::Infinite { negative }) => {
            if left.is_zero() {
                DecimalValue::not_a_number()
            } else {
                DecimalValue::infinity(negative != left.is_negative())
            }
        }
        (Kind::Infinite { negative: a }, Kind::Infinite { negative: b }) => {
            DecimalValue::infinity(a != b)
        }
        (Kind::Ordinary, Kind::Ordinary) => {
            let (Some(a), Some(b)) = (Scaled::from(left), Scaled::from(right)) else {
                return DecimalValue::not_a_number();
            };
            Scaled {
                value: a.value * b.value,
                exponent: a.exponent.saturating_add(b.exponent),
            }
            .into_decimal()
        }
    }
}

/// Division, carried to twenty places and rounded half away from zero.
#[must_use]
pub fn divide(left: &DecimalValue, right: &DecimalValue) -> DecimalValue {
    match (kind(left), kind(right)) {
        // Not a number spreads through everything, and an infinity divided by
        // an infinity has no value of its own either.
        (Kind::NotANumber, _)
        | (_, Kind::NotANumber)
        | (Kind::Infinite { .. }, Kind::Infinite { .. }) => DecimalValue::not_a_number(),
        (Kind::Infinite { negative }, Kind::Ordinary) => {
            DecimalValue::infinity(negative != right.is_negative())
        }
        (Kind::Ordinary, Kind::Infinite { .. }) => DecimalValue::zero(),
        (Kind::Ordinary, Kind::Ordinary) => {
            if right.is_zero() {
                // Zero over zero has no value; anything else over zero is an
                // infinity rather than a failure.
                if left.is_zero() {
                    return DecimalValue::not_a_number();
                }
                return DecimalValue::infinity(left.is_negative() != right.is_negative());
            }
            let (Some(a), Some(b)) = (Scaled::from(left), Scaled::from(right)) else {
                return DecimalValue::not_a_number();
            };
            quotient(&a, &b)
        }
    }
}

/// The rounded quotient of two scaled integers.
///
/// The result is wanted with one more place than is kept, so that the extra
/// digit can decide the rounding. Writing that out: the quotient is
/// `(a * 10^ae) / (b * 10^be)`, and expressing it as `R * 10^-p` gives
/// `R = a * 10^(ae - be + p) / b`. The exponent difference can point either
/// way, so the power lands on whichever side of the division keeps it whole.
fn quotient(a: &Scaled, b: &Scaled) -> DecimalValue {
    // The scale of the answer is known before any of its digits are: a
    // quotient sits at the difference of the two scales, give or take one.
    // Outside the model's range it is an infinity or a zero whatever those
    // digits turn out to be, and reaching that conclusion through the division
    // would first build a power of ten with millions of them.
    let limit = DecimalValue::EXPONENT_LIMIT.saturating_add(2);
    let scale = a.scale().saturating_sub(b.scale());
    if scale > limit {
        return DecimalValue::infinity(a.value.is_negative() != b.value.is_negative());
    }
    if scale < -limit {
        return DecimalValue::zero();
    }

    let places = DECIMAL_PLACES.saturating_add(1);
    let shift = a.exponent.saturating_sub(b.exponent).saturating_add(places);
    let power = BigInt::from(power_of_ten(shift.unsigned_abs()));
    let (numerator, denominator) = if shift >= 0 {
        (a.value.clone() * power, b.value.clone())
    } else {
        (a.value.clone(), b.value.clone() * power)
    };
    // One guard digit is enough here, and exactly enough. The division above
    // truncates, so that digit is the true twenty-first place; whether what
    // follows it is zero or not, a digit of five or more rounds away from zero
    // and a digit below five does not.
    round_last_digit(&(numerator / denominator), -places)
}

/// Drops the guard digit, rounding half away from zero.
fn round_last_digit(value: &BigInt, exponent: i64) -> DecimalValue {
    let negative = value.is_negative();
    let magnitude = value.magnitude().clone();
    let ten = BigUint::from(10_u32);
    let quotient = &magnitude / &ten;
    let remainder = &magnitude % &ten;
    // Half rounds away from zero, which is mode four: five and above lift the
    // kept digit whichever side of zero the value is on.
    let rounded = if remainder >= BigUint::from(5_u32) {
        quotient + BigUint::one()
    } else {
        quotient
    };
    DecimalValue::from_parts(negative, &rounded.to_string(), exponent.saturating_add(1))
}

/// The remainder of a truncated division, which takes the dividend's sign.
#[must_use]
pub fn modulo(left: &DecimalValue, right: &DecimalValue) -> DecimalValue {
    match (kind(left), kind(right)) {
        // An infinity has no remainder to give, so it joins not-a-number on
        // the left; on the right it leaves the dividend untouched below.
        (Kind::NotANumber | Kind::Infinite { .. }, _) | (_, Kind::NotANumber) => {
            DecimalValue::not_a_number()
        }
        (Kind::Ordinary, Kind::Infinite { .. }) => left.clone(),
        (Kind::Ordinary, Kind::Ordinary) => {
            if right.is_zero() {
                return DecimalValue::not_a_number();
            }
            let (Some(a), Some(b)) = (Scaled::from(left), Scaled::from(right)) else {
                return DecimalValue::not_a_number();
            };
            let exponent = a.exponent.min(b.exponent);
            let first = a.rescaled(exponent);
            let second = b.rescaled(exponent);
            // Rust's remainder already truncates toward zero, which is the
            // mode the library is configured with -- so `-7 % 3` is `-1` on
            // both sides rather than the `2` a flooring remainder would give.
            Scaled {
                value: first % second,
                exponent,
            }
            .into_decimal()
        }
    }
}

/// The square root, carried to twenty places.
#[must_use]
pub fn square_root(value: &DecimalValue) -> DecimalValue {
    match kind(value) {
        Kind::NotANumber => DecimalValue::not_a_number(),
        // The root of a negative has no value; the root of a positive infinity
        // is one.
        Kind::Infinite { negative } => {
            if negative {
                DecimalValue::not_a_number()
            } else {
                DecimalValue::infinity(false)
            }
        }
        Kind::Ordinary => {
            if value.is_negative() && !value.is_zero() {
                return DecimalValue::not_a_number();
            }
            let Some(scaled) = Scaled::from(value) else {
                return DecimalValue::not_a_number();
            };
            if scaled.value.is_zero() {
                return DecimalValue::zero();
            }
            root(&scaled)
        }
    }
}

/// A square root rounded at the twentieth place by an exact comparison.
///
/// Written out: the value is `m * 10^e` and the answer is wanted as `R * 10^-d`
/// with `d` places kept. Squaring both sides gives `R = sqrt(m * 10^(e + 2d))`,
/// so the radicand carries that power. The power can be negative -- a value
/// below `10^-2d` -- and a negative power of ten is not a whole number, so a
/// further `c` is lifted out of the root and divided off afterwards. Dividing
/// a *truncated* root is safe because `floor(floor(x) / n)` is `floor(x / n)`.
///
/// The rounding is not read off a guard digit, which is where a port of this
/// goes wrong. An integer root is truncated, so its last digit does not say
/// whether the true root lies above or below the half. The half is squared
/// instead: the answer is `R + 1` exactly when `4 * radicand >= (2R + 1)^2 *
/// 10^2c` -- a comparison of whole numbers, settling even an exact tie without
/// one approximate step.
fn root(scaled: &Scaled) -> DecimalValue {
    let places = DECIMAL_PLACES;
    // Below this the root falls under half of the last place kept, so the
    // answer is zero: the root halves the scale, hence twice the places, and
    // a little more for the slack in the estimate. Reaching the same answer
    // through the arithmetic would first build a power of ten with millions of
    // digits.
    if scaled.scale() < places.saturating_mul(2).saturating_add(4).saturating_neg() {
        return DecimalValue::zero();
    }

    let shifted = scaled.exponent.saturating_add(places.saturating_mul(2));
    let outside = if shifted < 0 {
        shifted.saturating_neg().saturating_add(1) / 2
    } else {
        0
    };
    let inside = shifted.saturating_add(outside.saturating_mul(2));

    let radicand = scaled.value.magnitude().clone() * power_of_ten(inside.unsigned_abs());
    let divisor = power_of_ten(outside.unsigned_abs());
    let kept = integer_root(&radicand) / &divisor;

    let half = (&kept * 2_u32 + BigUint::one()) * divisor;
    let rounded = if radicand * 4_u32 >= &half * &half {
        kept + BigUint::one()
    } else {
        kept
    };
    DecimalValue::from_parts(false, &rounded.to_string(), -places)
}

/// The largest integer whose square does not exceed `value`.
///
/// Newton's method on integers: each step is exact, and the sequence descends
/// to the answer rather than approaching it, so there is no tolerance to pick.
fn integer_root(value: &BigUint) -> BigUint {
    if value.is_zero() {
        return BigUint::zero();
    }
    let mut guess = BigUint::one() << ((value.bits() / 2) + 1);
    loop {
        let next = (&guess + value / &guess) >> 1_u32;
        if next >= guess {
            return guess;
        }
        guess = next;
    }
}

/// The value with its sign flipped.
///
/// Zero has no sign to flip: the reference renders a negated zero as `0`, and
/// the model drops the sign on zero for exactly that reason.
#[must_use]
pub fn negate(value: &DecimalValue) -> DecimalValue {
    match kind(value) {
        Kind::NotANumber => DecimalValue::not_a_number(),
        Kind::Infinite { negative } => DecimalValue::infinity(!negative),
        Kind::Ordinary => match value.parts() {
            Some((negative, digits, exponent)) => {
                DecimalValue::from_parts(!negative, digits, exponent)
            }
            None => DecimalValue::not_a_number(),
        },
    }
}

/// The value without its sign.
#[must_use]
pub fn absolute(value: &DecimalValue) -> DecimalValue {
    match kind(value) {
        Kind::NotANumber => DecimalValue::not_a_number(),
        Kind::Infinite { .. } => DecimalValue::infinity(false),
        Kind::Ordinary => match value.parts() {
            Some((_, digits, exponent)) => DecimalValue::from_parts(false, digits, exponent),
            None => DecimalValue::not_a_number(),
        },
    }
}

/// The total of a list, which is exact however long the list is.
///
/// Folding rather than a running float: every partial sum keeps all of its
/// digits, so a thousand tenths come to exactly a hundred.
///
/// An empty list totals zero here where the reference's fold, which has no
/// seed, produces nothing at all. The operations that call this decide what an
/// empty input means before they get here, so the difference never shows.
#[must_use]
pub fn sum(values: &[DecimalValue]) -> DecimalValue {
    let mut total = DecimalValue::zero();
    for value in values {
        total = plus(&total, value);
    }
    total
}

/// The mean, which is a sum and a division and therefore rounds.
#[must_use]
pub fn mean(values: &[DecimalValue]) -> DecimalValue {
    if values.is_empty() {
        return DecimalValue::not_a_number();
    }
    let count = DecimalValue::from(i128::try_from(values.len()).unwrap_or(i128::MAX));
    divide(&sum(values), &count)
}

/// The median, which is the middle value or the mean of the middle two.
#[must_use]
pub fn median(values: &[DecimalValue]) -> DecimalValue {
    if values.is_empty() {
        return DecimalValue::not_a_number();
    }
    let mut ordered: Vec<&DecimalValue> = values.iter().collect();
    ordered.sort_by(|left, right| compare(left, right));
    let middle = ordered.len() / 2;
    if ordered.len() % 2 == 1 {
        return ordered[middle].clone();
    }
    let pair = [ordered[middle - 1].clone(), ordered[middle].clone()];
    mean(&pair)
}

/// Orders two values, putting not-a-number last so a sort stays total.
fn compare(left: &DecimalValue, right: &DecimalValue) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    match (Scaled::from(left), Scaled::from(right)) {
        (Some(a), Some(b)) => {
            let exponent = a.exponent.min(b.exponent);
            a.rescaled(exponent).cmp(&b.rescaled(exponent))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
