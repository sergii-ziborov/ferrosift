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

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ferrosift_model::DecimalValue;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::jscompat::delim::is_js_whitespace;

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

/// `base` raised to `exponent`.
///
/// By repeated squaring rather than a loop of multiplications: the powers here
/// reach the model's exponent range, and ten million separate multiplications
/// would be the slowest thing in the crate by a wide margin.
fn power(base: u32, exponent: u64) -> BigUint {
    // Every caller bounds the exponent by the model's range plus the digits of
    // its operands, so the narrowing below has nothing to lose. Saturating
    // rather than unwrapping keeps the function total.
    BigUint::from(base).pow(u32::try_from(exponent).unwrap_or(u32::MAX))
}

/// Ten raised to `power`.
fn power_of_ten(power: u64) -> BigUint {
    self::power(10, power)
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
    // Zero is the one operand that has to be short-circuited rather than
    // rescaled. Bringing `1e+5000000` down to zero's exponent materialises five
    // million digits to add nothing to them, which is why `Mean` over
    // `1e+5000000` and `0` spent nine seconds on an addition whose answer is
    // its own first operand. `sum_min_len` reports no cost for a zero operand,
    // which was true of the answer and false of the work — so this makes the
    // work match what was already being claimed about it.
    if right.is_zero() {
        return left.clone();
    }
    if left.is_zero() {
        return if subtract {
            negate(right)
        } else {
            right.clone()
        };
    }
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

/// A floor on how long the sum or difference of these two must render.
///
/// Exact addition is the one operation here that turns short inputs into a long
/// answer: `1e10000000 + 1e-10000000` is twenty-three characters in and twenty
/// million digits out, and the digits have to exist before anything can measure
/// them. Five seconds of that work, on a budget that then refuses the answer, is
/// the thing this exists to avoid — so a caller with a ceiling can ask first.
///
/// A *floor*, and the direction matters: acting on it means refusing, so an
/// estimate that ever came in high would refuse an answer the budget would have
/// accepted. Zero means no claim, which is the honest answer whenever the two
/// exponents are equal — there is no rescaling to pay for in that case anyway.
///
/// Two facts make a floor possible. A coefficient never ends in a zero, because
/// the model moves trailing zeros into the exponent, so the operand with the
/// lower exponent has a non-zero digit where the other has nothing at all: that
/// digit survives into the answer and fixes its bottom. And a value whose scale
/// is more than one above the other's cannot have its top cancelled, because
/// there is nothing up there to cancel against — which fixes the answer's top
/// whenever the two are far apart, and claims nothing when they are close.
#[must_use]
pub fn sum_min_len(left: &DecimalValue, right: &DecimalValue) -> u64 {
    if left.is_zero() || right.is_zero() {
        return 0;
    }
    let (Some((_, left_digits, left_exponent)), Some((_, right_digits, right_exponent))) =
        (left.parts(), right.parts())
    else {
        // A special answers a special, which is three characters at most.
        return 0;
    };
    if left_exponent == right_exponent {
        return 0;
    }

    let bottom = left_exponent.min(right_exponent);
    let left_scale = scale_of(left_digits, left_exponent);
    let right_scale = scale_of(right_digits, right_exponent);
    // More than one apart, so the larger one's leading digits have nothing to
    // cancel against and the answer reaches at least one place below its scale.
    let top = if left_scale.abs_diff(right_scale) > 1 {
        left_scale.max(right_scale).saturating_sub(1)
    } else {
        bottom
    };

    // Digits above the point, digits below it, and the one character every
    // rendering has whether or not either side is occupied.
    let above = u64::try_from(top.max(0)).unwrap_or(u64::MAX);
    let below = u64::try_from(bottom.min(0).saturating_neg()).unwrap_or(u64::MAX);
    above.saturating_add(below).saturating_add(1)
}

/// A floor on how long `left / right` renders.
///
/// Division was left unguarded on the reasoning that it "already refuses an
/// out-of-range scale before it computes any digits" — true, and not the whole
/// story. The refusal is against [`DecimalValue::EXPONENT_LIMIT`], ten
/// million, so a scale of five million is *in* range and computed in full:
/// `1e+5000000 / 3` spends thirty-four seconds producing a five-million-digit
/// answer, which the executor then refuses for being five million digits long.
/// A floor is what turns that into an immediate answer.
///
/// The quotient's leading digit sits one or zero places below the difference
/// of the two scales, depending on which coefficient is larger, so the
/// difference itself is the count of digits above the point that is certainly
/// reached. Claiming the difference plus one would be an over-estimate on
/// every pair where the divisor's coefficient wins, and an over-estimating
/// floor refuses answers the budget would have accepted.
#[must_use]
pub fn quotient_min_len(left: &DecimalValue, right: &DecimalValue) -> u64 {
    if left.is_zero() || right.is_zero() {
        return 0;
    }
    let (Some((_, left_digits, left_exponent)), Some((_, right_digits, right_exponent))) =
        (left.parts(), right.parts())
    else {
        return 0;
    };
    let scale =
        scale_of(left_digits, left_exponent).saturating_sub(scale_of(right_digits, right_exponent));
    if scale <= 0 {
        // A quotient below one renders as `0.` and the places the division
        // keeps, which is a constant and not worth claiming.
        return 0;
    }
    u64::try_from(scale).unwrap_or(u64::MAX)
}

/// A floor on how long the square root of `value` renders.
///
/// Wanted for the same reason [`sum_min_len`] is, and it took a fuzzer to
/// notice the difference. Addition was guarded and the root was not, so
/// `Standard Deviation` over `1e+5000000` computed a five-million-digit answer
/// and *then* had it refused by the output ceiling: the right verdict, reached
/// after twenty-nine seconds of work the verdict says should not have
/// happened. `root` gets there by building a radicand with `exponent + 40`
/// digits, so the cost is set by the exponent and is invisible to every other
/// limit.
///
/// A floor, like its neighbour: acting on it means refusing, so it must never
/// come in high. The root of a value whose leading digit sits at `10^n` sits
/// at `10^(n/2)`, which is `n/2 + 1` digits above the point — exactly, by
/// truncating division, for every `n` at or above zero. Below zero the root is
/// under one and nothing is claimed.
#[must_use]
pub fn root_min_len(value: &DecimalValue) -> u64 {
    if value.is_zero() {
        return 0;
    }
    let Some((_, digits, exponent)) = value.parts() else {
        // A special roots to a special, which is three characters at most.
        return 0;
    };
    let scale = scale_of(digits, exponent);
    if scale < 0 {
        return 0;
    }
    u64::try_from(scale / 2)
        .unwrap_or(u64::MAX)
        .saturating_add(1)
}

/// The power of ten a value sits at, read off its parts rather than computed.
///
/// This is the same quantity [`Scaled::scale`] estimates from the binary width,
/// and here it is exact and free: the model normalises a coefficient so that
/// its length and exponent say it directly.
fn scale_of(digits: &str, exponent: i64) -> i64 {
    let length = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    exponent.saturating_add(length).saturating_sub(1)
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
            // Nothing divided by anything is nothing, and saying so here is
            // what stops the division below building a power of ten to
            // multiply a zero by: `0 / 1e-10000000` reaches a shift of ten
            // million, which is ten million digits produced to be discarded.
            if left.is_zero() {
                return DecimalValue::zero();
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
    // And the same conclusion from the other end, which the range check above
    // is far too wide to reach. A quotient whose leading digit sits below the
    // guard digit rounds to zero whatever follows it, so `3 / 1e9999999` is
    // zero — but the range check only refuses a scale past ten million, so
    // this was computed by building a power of ten with ten million digits to
    // divide it away again. A fuzzer found it as a forty-four-second
    // execution. The margin covers `scale`, which is estimated from the binary
    // width and is good to within a digit either way.
    if scale < places.saturating_add(4).saturating_neg() {
        return DecimalValue::zero();
    }

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

            // A dividend smaller than the modulus is its own remainder, and
            // saying so from the parts is what avoids the whole problem: the
            // alternative brings both to a common exponent, and the exponent
            // gap is exactly what makes that expensive. `1e-10000000 MOD 2`
            // would otherwise scale the *modulus* by ten million places to
            // conclude that the dividend was already the answer.
            if magnitude_below(left, right) {
                return left.clone();
            }

            // The dividend's exponent above the modulus's is a power of ten
            // that never has to exist. `ca * 10^k mod m` is
            // `(ca mod m) * (10^k mod m) mod m`, and the second factor is a
            // modular exponentiation -- so `1e10000000 MOD 2` is a few dozen
            // multiplications rather than a ten-million-digit integer built to
            // be divided away.
            if a.exponent > b.exponent {
                let shift = a.exponent.abs_diff(b.exponent);
                let modulus = b.value.magnitude().clone();
                let base = a.value.magnitude() % &modulus;
                let power = BigUint::from(10_u32).modpow(&BigUint::from(shift), &modulus);
                let reduced = base * power % &modulus;
                let sign = if a.value.is_negative() {
                    Sign::Minus
                } else {
                    Sign::Plus
                };
                return Scaled {
                    value: BigInt::from_biguint(sign, reduced),
                    exponent: b.exponent,
                }
                .into_decimal();
            }

            // What is left is a modulus with the finer exponent, and a
            // dividend at least as large -- so the shift below is bounded by
            // the dividend's own digits rather than by the gap.
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

/// Whether `left` is smaller than `right` in magnitude.
///
/// Read off the parts rather than by aligning the two, for the same reason
/// [`compare`] is: the alignment costs the gap between the exponents, and the
/// question is usually settled by the leading positions alone. A scale is
/// exact here — the model normalises a coefficient so its length and exponent
/// give it directly — so the comparison it settles is exact too.
///
/// Only when the scales agree do the digits get compared, and then the shift is
/// bounded by the coefficients themselves rather than by the exponent gap.
fn magnitude_below(left: &DecimalValue, right: &DecimalValue) -> bool {
    if left.is_zero() {
        return !right.is_zero();
    }
    if right.is_zero() {
        return false;
    }
    let (Some((_, left_digits, left_exponent)), Some((_, right_digits, right_exponent))) =
        (left.parts(), right.parts())
    else {
        return false;
    };
    match scale_of(left_digits, left_exponent).cmp(&scale_of(right_digits, right_exponent)) {
        core::cmp::Ordering::Less => true,
        core::cmp::Ordering::Greater => false,
        core::cmp::Ordering::Equal => {
            let (Some(a), Some(b)) = (Scaled::from(left), Scaled::from(right)) else {
                return false;
            };
            let exponent = a.exponent.min(b.exponent);
            a.rescaled(exponent).magnitude() < b.rescaled(exponent).magnitude()
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

/// The lowest and highest base the reference will read or write.
const BASES: core::ops::RangeInclusive<u32> = 2..=36;

/// How many places a base conversion keeps, as a count.
fn places() -> usize {
    usize::try_from(DECIMAL_PLACES).unwrap_or(0)
}

/// Reads a number written in another base, or `None` where the reference
/// refuses one.
///
/// A different set of rules from the single-argument reading, and several of
/// them run backwards from it, which is why both are pinned rather than one
/// being derived from the other:
///
/// - an empty string is zero here, where the other refuses it;
/// - `NaN` and `Infinity` are refused here, where the other reads them;
/// - a `0x` prefix is refused here, where the other reads it;
/// - `e` is a digit rather than an exponent marker, so `1e5` in base sixteen
///   is four hundred and eighty-five rather than a hundred thousand;
/// - the letters must agree on their case. The reference matches the whole
///   string against one alphabet, so `ff` and `FF` are both read and `Ff` is
///   not -- a rule with no analogue in the ordinary reading, where case never
///   mattered at all.
///
/// The value is read as one whole number over a power of the base -- `1010.1011`
/// in base two is `171 / 2^4` -- so a fraction that terminates in the base comes
/// out exact, and one that does not is rounded once at the twentieth place
/// rather than accumulating a rounding per digit.
#[must_use]
pub fn parse_in_base(text: &str, base: u32) -> Option<DecimalValue> {
    if !BASES.contains(&base) {
        return None;
    }
    let trimmed = text.trim_matches(is_js_whitespace);
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };

    let (whole, fraction) = match rest.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (rest, ""),
    };

    // A second point is not a digit in any base, so it fails here rather than
    // needing a rule of its own -- and so does every other stray character.
    let mut digits = String::with_capacity(whole.len() + fraction.len());
    let (mut lower, mut upper) = (false, false);
    for character in whole.chars().chain(fraction.chars()) {
        let value = character.to_digit(36)?;
        if value >= base {
            return None;
        }
        lower |= character.is_ascii_lowercase();
        upper |= character.is_ascii_uppercase();
        digits.push(character);
    }
    // One alphabet or the other, never both. The point does not divide them:
    // `1f.A` is refused as surely as `Ff` is.
    if lower && upper {
        return None;
    }

    // No digits at all is zero rather than a refusal, which is the surprise
    // worth stating: `new BigNumber("", 16)` is zero where `new BigNumber("")`
    // throws.
    if digits.is_empty() {
        return Some(DecimalValue::zero());
    }

    let magnitude = BigUint::parse_bytes(digits.as_bytes(), base)?;
    let numerator = DecimalValue::from_parts(negative, &magnitude.to_string(), 0);
    if fraction.is_empty() {
        return Some(numerator);
    }
    let scale = power(base, fraction.len() as u64);
    Some(divide(
        &numerator,
        &DecimalValue::from_parts(false, &scale.to_string(), 0),
    ))
}

/// Writes the value in another base, or `None` for a base outside the range.
///
/// Not the same as either rendering in the model, and the difference is worth
/// naming: this one *never* uses exponential notation, not even for base ten,
/// where the argumentless `toString` does. `1e21` written in base ten comes out
/// as twenty-two characters.
///
/// The fraction is carried to twenty places **of the target base** rather than
/// of ten -- a third in base two is twenty binary digits -- and the rounding is
/// decided by the twenty-first digit alone, against half the base. On an odd
/// base that means an exact tie rounds *down*, which is the opposite of what
/// every other rounding in this module does.
#[must_use]
pub fn to_base(value: &DecimalValue, base: u32) -> Option<String> {
    if !BASES.contains(&base) {
        return None;
    }
    if value.is_not_a_number() {
        return Some(String::from("NaN"));
    }
    if value.is_infinite() {
        return Some(String::from(if value.is_negative() {
            "-Infinity"
        } else {
            "Infinity"
        }));
    }

    let (negative, digits, exponent) = value.parts()?;
    if digits.is_empty() {
        return Some(String::from("0"));
    }
    let magnitude = BigUint::parse_bytes(digits.as_bytes(), 10)?;

    // The magnitude as an exact fraction: `whole + remainder / divisor`. A
    // positive exponent has no fractional part at all, which is most inputs.
    let (mut whole, remainder, divisor) = if exponent >= 0 {
        (
            magnitude * power_of_ten(exponent.unsigned_abs()),
            BigUint::zero(),
            BigUint::one(),
        )
    } else {
        let divisor = power_of_ten(exponent.unsigned_abs());
        let whole = &magnitude / &divisor;
        let remainder = magnitude % &divisor;
        (whole, remainder, divisor)
    };

    // One digit further than is kept, because that digit alone decides the
    // rounding.
    let mut fraction = Vec::new();
    let mut left = remainder;
    let radix = BigUint::from(base);
    for _ in 0..=DECIMAL_PLACES {
        if left.is_zero() {
            break;
        }
        left *= &radix;
        fraction.push(digit_of(&(&left / &divisor)));
        left %= &divisor;
    }

    // The reference compares that digit against half the base as a *real*
    // number, and looks at nothing after it. Two consequences, and the second
    // is the one a port gets wrong:
    //
    // - a tail that continues past the deciding digit does not lift a value
    //   the digit itself leaves below the half;
    // - on an odd base no digit is worth exactly half, so an exact tie rounds
    //   *down*. A tenth in base five repeats as `0.0222...`, sits exactly half
    //   a place above the twentieth digit, and comes out truncated -- where
    //   the same tie in base sixteen rounds away from zero.
    //
    // Written as `2 * digit >= base` because that is both conditions at once:
    // equality is reachable only when the base is even.
    if fraction.len() > places() {
        let deciding = fraction.pop().unwrap_or(0);
        if u64::from(deciding) * 2 >= u64::from(base) {
            // The carry can run off the front of the fraction into the whole
            // part, which is why the whole part is rendered after this.
            carry_one(&mut fraction, &mut whole, base);
        }
    }
    while fraction.last() == Some(&0) {
        fraction.pop();
    }

    // A value that rounded away to nothing is zero, and zero carries no sign.
    if whole.is_zero() && fraction.is_empty() {
        return Some(String::from("0"));
    }

    let mut output = String::new();
    if negative {
        output.push('-');
    }
    output.push_str(&whole.to_str_radix(base));
    if !fraction.is_empty() {
        output.push('.');
        for digit in fraction {
            output.push(char::from_digit(digit, base).unwrap_or('0'));
        }
    }
    Some(output)
}

/// `base` raised to `exponent`, as a value rather than as an integer.
///
/// The reference reaches this through `pow`, which is exact for a whole
/// exponent -- so nothing here rounds, and a caller that divides by the result
/// rounds once rather than twice.
#[must_use]
pub fn power_of(base: u32, exponent: u64) -> DecimalValue {
    DecimalValue::from_parts(false, &power(base, exponent).to_string(), 0)
}

/// One digit of a base, from a value the division above bounded below it.
fn digit_of(value: &BigUint) -> u32 {
    value.to_u32().unwrap_or(0)
}

/// Adds one to the last place, carrying into the whole part if it runs out.
fn carry_one(fraction: &mut [u32], whole: &mut BigUint, base: u32) {
    for digit in fraction.iter_mut().rev() {
        if *digit + 1 < base {
            *digit += 1;
            return;
        }
        *digit = 0;
    }
    *whole += BigUint::one();
}

/// The values in the order a median reads them.
///
/// Exposed because a caller with a resource ceiling needs the sorted list
/// without the averaging step: the middle pair is a sum, and a sum is where the
/// width comes from. Sharing the sort keeps the two from disagreeing about
/// which values are in the middle.
#[must_use]
pub fn ordered(values: &[DecimalValue]) -> Vec<DecimalValue> {
    let mut sorted = values.to_vec();
    sorted.sort_by(compare);
    sorted
}

/// The median, which is the middle value or the mean of the middle two.
#[must_use]
pub fn median(values: &[DecimalValue]) -> DecimalValue {
    if values.is_empty() {
        return DecimalValue::not_a_number();
    }
    let sorted = ordered(values);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        return sorted[middle].clone();
    }
    mean(&sorted[middle - 1..=middle])
}

/// Orders two values, putting not-a-number last so a sort stays total.
///
/// Magnitude first, from the parts, and only then by the digits. Bringing both
/// to a common exponent is what settles two values of the same size, and it
/// costs the gap between their exponents -- which for `1e10000000` beside
/// `1e-10000000` is twenty million digits built to answer a question their
/// leading positions had already answered. Where the scales differ, the larger
/// one is larger: it has a digit in a place the other cannot reach.
fn compare(left: &DecimalValue, right: &DecimalValue) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    match (Scaled::from(left), Scaled::from(right)) {
        (Some(a), Some(b)) => {
            let signs = a.value.sign().cmp(&b.value.sign());
            if signs != Ordering::Equal {
                return signs;
            }
            if let (
                Some((_, left_digits, left_exponent)),
                Some((_, right_digits, right_exponent)),
            ) = (left.parts(), right.parts())
            {
                let by_scale = scale_of(left_digits, left_exponent)
                    .cmp(&scale_of(right_digits, right_exponent));
                // A negative pair orders by magnitude the other way round.
                let by_scale = if a.value.is_negative() {
                    by_scale.reverse()
                } else {
                    by_scale
                };
                if by_scale != Ordering::Equal {
                    return by_scale;
                }
            }
            let exponent = a.exponent.min(b.exponent);
            a.rescaled(exponent).cmp(&b.rescaled(exponent))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
