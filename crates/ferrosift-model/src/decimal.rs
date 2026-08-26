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

    /// Whether this is an infinity, of either sign.
    #[must_use]
    pub fn is_infinite(&self) -> bool {
        self.special == Some(DecimalSpecial::Infinite)
    }

    /// Whether the value carries a minus sign.
    ///
    /// False for zero, which the reference renders without one however it was
    /// written.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// Whether the value is exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.special.is_none() && self.digits.is_empty()
    }

    /// The sign, the coefficient's digits, and its power of ten.
    ///
    /// `None` for a value that is not an ordinary number. Exposed so that an
    /// arithmetic backend can work on the coefficient rather than on the
    /// rendering: going through `to_fixed` would turn a value with a large
    /// exponent into millions of characters before doing anything with it.
    #[must_use]
    pub fn parts(&self) -> Option<(bool, &str, i64)> {
        if self.special.is_some() {
            return None;
        }
        Some((self.negative, self.digits.as_str(), self.exponent))
    }

    /// Builds a value from a sign, digits, and a power of ten.
    ///
    /// The result is normalised and range-checked exactly as parsed input is,
    /// so an arithmetic backend cannot produce a value the parser could not.
    #[must_use]
    pub fn from_parts(negative: bool, digits: &str, exponent: i64) -> Self {
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Self::not_a_number();
        }
        Self::normalised(negative, digits, exponent)
    }

    /// Positive or negative infinity, for a backend that produced one.
    #[must_use]
    pub fn infinity(negative: bool) -> Self {
        Self::infinite(negative)
    }

    /// Reads a decimal, answering not-a-number for anything unreadable.
    ///
    /// The reference's constructor *throws* on input it cannot read; the dish
    /// catches and substitutes not-a-number. What a recipe observes is the
    /// substitution, so that is what this returns -- a failing constructor
    /// would report an error the reference never surfaces.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let trimmed = trim_like_the_reference(input);
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

        // A base prefix is read by the single-argument constructor, so `0x1f`
        // is thirty-one rather than unreadable. Checked before the exponent
        // split, because `0b101` contains no `e` but `0xe` does.
        if let Some(value) = parse_prefixed_base(negative, rest) {
            return value;
        }

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

    /// The largest and smallest exponent the reference keeps.
    ///
    /// Beyond it the value becomes infinite, and below it zero. The limit is
    /// on the *normalised* exponent -- the power of ten beside a single
    /// leading digit -- so `100e9999998` and `1e10000000` are the same value
    /// and both survive, while `1e10000001` does not.
    ///
    /// Not a detail. Without the clamp a three-character source could describe
    /// a number whose rendering is unbounded, and the budget that is supposed
    /// to refuse it would have to render it to find out.
    ///
    /// Public because arithmetic wants to know the range *before* it works.
    /// A quotient whose scale lands outside this is infinite or zero whatever
    /// its digits are, and computing those digits first would mean building a
    /// power of ten with millions of them only to throw it away.
    pub const EXPONENT_LIMIT: i64 = 10_000_000;

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

        // The normalised exponent: the power of ten beside a single leading
        // digit, which is what the reference's range applies to.
        let length = i64::try_from(kept.len()).unwrap_or(i64::MAX);
        let normalised = exponent.saturating_add(length).saturating_sub(1);
        if normalised > Self::EXPONENT_LIMIT {
            return Self::infinite(negative);
        }
        if normalised < -Self::EXPONENT_LIMIT {
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

    /// The exponents at which the reference's `toString` turns exponential:
    /// at or below the first, at or above the second.
    ///
    /// Read from the library rather than from its documentation, which gives
    /// the positive threshold as twenty where the code uses twenty-one -- the
    /// same kind of error as the exponent range, which the documentation puts
    /// at a billion and the code at ten million. `1e20` is written out in full
    /// and `1e21` is not.
    const NOTATION_RANGE: (i64, i64) = (-7, 21);

    /// Renders the value the way the reference's `toString` does.
    ///
    /// Not the same as [`Self::to_fixed`], and the difference is why both
    /// exist. The dish converts with `toFixed`, which never uses exponential
    /// notation whatever the exponent. An operation that *joins* numbers into
    /// a string of its own gets `toString`, which does -- so a port carrying
    /// only one of these would be right about a remainder of `2.5` and wrong
    /// about a remainder of `1e-8`, in an operation whose other answers all
    /// looked correct.
    #[must_use]
    pub fn to_notation(&self) -> String {
        // The specials and zero read the same either way.
        if self.special.is_some() || self.digits.is_empty() {
            return self.to_fixed();
        }

        let length = i64::try_from(self.digits.len()).unwrap_or(i64::MAX);
        // The exponent beside a single leading digit, which is the quantity
        // the thresholds apply to.
        let normalised = self.exponent.saturating_add(length).saturating_sub(1);
        let (negative_at, positive_at) = Self::NOTATION_RANGE;
        if normalised > negative_at && normalised < positive_at {
            return self.to_fixed();
        }

        let mut output = String::new();
        if self.negative {
            output.push('-');
        }
        output.push_str(&self.digits[..1]);
        if self.digits.len() > 1 {
            output.push('.');
            output.push_str(&self.digits[1..]);
        }
        output.push('e');
        // A negative exponent brings its own sign; a positive one is written
        // with a plus, which `toString` includes and a bare number would not.
        if normalised >= 0 {
            output.push('+');
        }
        output.push_str(&normalised.to_string());
        output
    }
}

/// Trims what the reference trims, which is not what Rust calls whitespace.
///
/// The two sets overlap but neither contains the other, and both differences
/// are reachable. A next-line character (U+0085) has the Unicode `White_Space`
/// property, so `str::trim` removes it -- and the reference does not, which
/// makes such input not-a-number there and a valid number here. A byte-order
/// mark does not have the property, so `str::trim` keeps it -- and the
/// reference removes it, which makes a spreadsheet's export readable there and
/// unreadable here.
fn trim_like_the_reference(input: &str) -> &str {
    input.trim_matches(is_reference_space)
}

/// One character of what the reference treats as leading or trailing space.
fn is_reference_space(character: char) -> bool {
    matches!(
        character,
        // Tab, line feed, vertical tab, form feed, carriage return, space.
        '\u{09}'..='\u{0d}' | '\u{20}'
        // No-break space, and the byte-order mark, which is not `White_Space`.
        | '\u{a0}' | '\u{feff}'
        // The two separators, and the space characters of category Zs.
        | '\u{2028}' | '\u{2029}'
        | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

/// Reads `0x`, `0b`, or `0o` the way the single-argument constructor does.
///
/// Returns `None` when the text carries no such prefix, which leaves the
/// decimal path to read it.
fn parse_prefixed_base(negative: bool, rest: &str) -> Option<DecimalValue> {
    let (radix, digits) = match rest.get(..2)? {
        "0x" | "0X" => (16, &rest[2..]),
        "0b" | "0B" => (2, &rest[2..]),
        "0o" | "0O" => (8, &rest[2..]),
        _ => return None,
    };
    if digits.is_empty() {
        return Some(DecimalValue::not_a_number());
    }
    let mut decimal: Vec<u8> = alloc::vec![0];
    for character in digits.chars() {
        let Some(value) = character.to_digit(radix) else {
            return Some(DecimalValue::not_a_number());
        };
        // Long multiplication in base ten, least significant digit first, so
        // the result is exact however many digits the source carries.
        let mut carry = value;
        for slot in &mut decimal {
            let product = u32::from(*slot) * radix + carry;
            *slot = u8::try_from(product % 10).unwrap_or(0);
            carry = product / 10;
        }
        while carry > 0 {
            decimal.push(u8::try_from(carry % 10).unwrap_or(0));
            carry /= 10;
        }
    }
    let text: String = decimal
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + *digit))
        .collect();
    Some(DecimalValue::normalised(negative, &text, 0))
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
