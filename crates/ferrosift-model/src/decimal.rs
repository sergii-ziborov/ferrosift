//! An arbitrary-precision decimal, carried without an arbitrary-precision crate.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// A value that is not a number, or is larger than any.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecimalSpecial {
    /// Not a number.
    NotANumber,
    /// Positive or negative infinity, per the sign beside it.
    Infinite,
}

/// A decimal number of unbounded size and precision.
///
/// Held as a sign, a run of digits, and a power of ten rather than by
/// depending on an arbitrary-precision crate. The model is what every consumer
/// of a value has to compile, and which library does the arithmetic is a
/// choice only the arithmetic cares about -- so the representation is
/// canonical and the backend is free.
///
/// The form is normalised on construction: no leading zeros, no trailing zeros
/// carried in the coefficient, and no sign on zero. That is not tidiness. The
/// reference renders `1.000` as `1` and `-0` as `0`, so two values that print
/// the same must *be* the same here, or equality would disagree with output.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct DecimalValue {
    negative: bool,
    /// The coefficient's digits, most significant first. Empty means zero.
    digits: String,
    /// The power of ten the coefficient is scaled by.
    exponent: i64,
    /// Set when the value is not an ordinary number.
    special: Option<DecimalSpecial>,
}

impl DecimalValue {
    /// The zero value.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            negative: false,
            digits: String::new(),
            exponent: 0,
            special: None,
        }
    }

    /// A value that is not a number.
    #[must_use]
    pub fn not_a_number() -> Self {
        Self {
            negative: false,
            digits: String::new(),
            exponent: 0,
            special: Some(DecimalSpecial::NotANumber),
        }
    }

    /// Whether this is the not-a-number value.
    #[must_use]
    pub fn is_not_a_number(&self) -> bool {
        self.special == Some(DecimalSpecial::NotANumber)
    }

    /// Reads a decimal, answering not-a-number for anything unreadable.
    ///
    /// The reference's constructor *throws* on input it cannot read; the dish
    /// catches and substitutes not-a-number. What a recipe observes is the
    /// substitution, so that is what this returns -- a failing constructor
    /// would report an error the reference never surfaces.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        match trimmed {
            "NaN" => return Self::not_a_number(),
            "Infinity" | "+Infinity" => return Self::infinite(false),
            "-Infinity" => return Self::infinite(true),
            _ => {}
        }

        let (negative, rest) = match trimmed.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
        };

        let (mantissa, exponent_text) = match rest.find(['e', 'E']) {
            Some(at) => (&rest[..at], Some(&rest[at + 1..])),
            None => (rest, None),
        };

        let (whole, fraction) = match mantissa.split_once('.') {
            Some((whole, fraction)) => (whole, fraction),
            None => (mantissa, ""),
        };

        if mantissa.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            || (whole.is_empty() && fraction.is_empty())
        {
            return Self::not_a_number();
        }

        let mut exponent = -i64::try_from(fraction.len()).unwrap_or(i64::MAX);
        if let Some(text) = exponent_text {
            let Ok(written) = text.parse::<i64>() else {
                return Self::not_a_number();
            };
            exponent = exponent.saturating_add(written);
        }

        let mut digits = String::with_capacity(whole.len() + fraction.len());
        digits.push_str(whole);
        digits.push_str(fraction);
        Self::normalised(negative, &digits, exponent)
    }

    /// Positive or negative infinity.
    #[must_use]
    fn infinite(negative: bool) -> Self {
        Self {
            negative,
            digits: String::new(),
            exponent: 0,
            special: Some(DecimalSpecial::Infinite),
        }
    }

    /// Builds a value with its coefficient reduced to canonical form.
    fn normalised(negative: bool, digits: &str, exponent: i64) -> Self {
        let trimmed = digits.trim_start_matches('0');
        let leading = digits.len() - trimmed.len();
        let mut kept: Vec<u8> = digits.as_bytes()[leading..].to_vec();

        // Trailing zeros move into the exponent, so `1.000` and `1` become the
        // same value rather than two that happen to print alike.
        let mut exponent = exponent;
        while kept.last() == Some(&b'0') {
            kept.pop();
            exponent = exponent.saturating_add(1);
        }

        if kept.is_empty() {
            return Self::zero();
        }
        Self {
            negative,
            digits: String::from_utf8(kept).unwrap_or_default(),
            exponent,
            special: None,
        }
    }

    /// How many bytes [`DecimalValue::to_fixed`] would produce, without
    /// producing them.
    ///
    /// This exists because measuring must not cost what it measures. A budget
    /// asks for the size of a value before deciding whether to allow it, and
    /// `1e100000000` is a tiny value -- one digit and an exponent -- whose
    /// rendering is a hundred megabytes. Measuring it by rendering it would
    /// make the allocation the budget exists to prevent, in order to discover
    /// that the budget forbids it.
    ///
    /// The arithmetic mirrors `to_fixed` branch for branch, so the two cannot
    /// drift apart without a test noticing.
    #[must_use]
    pub fn rendered_len(&self) -> u64 {
        match self.special {
            Some(DecimalSpecial::NotANumber) => return 3,
            Some(DecimalSpecial::Infinite) => {
                return if self.negative { 9 } else { 8 };
            }
            None => {}
        }
        if self.digits.is_empty() {
            return 1;
        }

        let digits = u64::try_from(self.digits.len()).unwrap_or(u64::MAX);
        let sign = u64::from(self.negative);
        if self.exponent >= 0 {
            // The digits, then that many zeros after them.
            return sign
                .saturating_add(digits)
                .saturating_add(self.exponent.unsigned_abs());
        }

        // A negative exponent moves the point left by that many places.
        let places = self.exponent.unsigned_abs();
        if places >= digits {
            // The point lands at or before the first digit, so the rendering is
            // `0.`, then the zeros that separate it from the digits.
            return sign
                .saturating_add(2)
                .saturating_add(places - digits)
                .saturating_add(digits);
        }
        // The point lands inside the digits, adding one character.
        sign.saturating_add(digits).saturating_add(1)
    }

    /// Renders the value the way the reference's dish does.
    ///
    /// The dish converts with `toFixed()` and no argument, which never uses
    /// exponential notation whatever the exponent -- so a value written `1e+25`
    /// comes out in full. Reproducing `toString` instead would be wrong in
    /// exactly the cases hardest to notice: the very large and the very small.
    #[must_use]
    pub fn to_fixed(&self) -> String {
        match self.special {
            Some(DecimalSpecial::NotANumber) => return String::from("NaN"),
            Some(DecimalSpecial::Infinite) => {
                return String::from(if self.negative {
                    "-Infinity"
                } else {
                    "Infinity"
                });
            }
            None => {}
        }
        if self.digits.is_empty() {
            // Zero carries no sign: the reference renders `-0` as `0`.
            return String::from("0");
        }

        let mut output = String::new();
        if self.negative {
            output.push('-');
        }
        if self.exponent >= 0 {
            output.push_str(&self.digits);
            for _ in 0..self.exponent {
                output.push('0');
            }
            return output;
        }

        let length = i64::try_from(self.digits.len()).unwrap_or(i64::MAX);
        let point = length + self.exponent;
        if point <= 0 {
            output.push_str("0.");
            for _ in 0..-point {
                output.push('0');
            }
            output.push_str(&self.digits);
        } else {
            let split = usize::try_from(point).unwrap_or(0);
            output.push_str(&self.digits[..split]);
            output.push('.');
            output.push_str(&self.digits[split..]);
        }
        output
    }
}

impl core::fmt::Display for DecimalValue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.to_fixed())
    }
}

impl From<i128> for DecimalValue {
    fn from(value: i128) -> Self {
        let negative = value < 0;
        let magnitude = value.unsigned_abs();
        Self::normalised(negative, &magnitude.to_string(), 0)
    }
}
