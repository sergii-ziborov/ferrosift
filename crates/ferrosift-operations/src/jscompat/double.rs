//! JavaScript's spelling of a number.
//!
//! `String(x)` is not "print the number". ECMA-262 takes the *shortest* digit
//! string that reads back as the same double, then chooses between plain and
//! exponential notation on two thresholds — above `1e21` and below `1e-6` —
//! that no other language shares.
//!
//! Rust agrees on the digits and disagrees on both thresholds: `1e21` prints
//! as `1000000000000000000000` here and `1e+21` there, and `1e-7` prints as
//! `0.0000001` here and `1e-7` there. So the digits are borrowed and the
//! notation is decided again.

use alloc::string::{String, ToString};

/// Above this many integer digits, the notation switches to exponential.
const PLAIN_UPPER: i32 = 21;

/// At or below this exponent, the notation switches to exponential.
///
/// Not the mirror of [`PLAIN_UPPER`]: the two thresholds are independent in the
/// specification and one is not derived from the other.
const PLAIN_LOWER: i32 = -6;

/// Formats a double the way JavaScript's `String(x)` does.
///
/// # Panics
///
/// Never: the exponent form Rust produces always contains its separator.
#[must_use]
pub(crate) fn format(value: f64) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value.is_infinite() {
        return String::from(if value < 0.0 { "-Infinity" } else { "Infinity" });
    }
    // Negative zero prints as "0": the sign is dropped rather than shown, and
    // only for this one value.
    if value == 0.0 {
        return String::from("0");
    }

    let negative = value < 0.0;
    let (digits, exponent) = shortest(value.abs());
    let mut text = spell(&digits, exponent);
    if negative {
        text.insert(0, '-');
    }
    text
}

/// The shortest round-tripping digits, and the exponent of the first one.
///
/// Rust's `{:e}` already produces shortest-round-trip digits — the same digits
/// ECMA-262 asks for, and unique, so borrowing them is exact rather than
/// approximate. Only the arrangement has to be redone.
fn shortest(value: f64) -> (String, i32) {
    let rendered = alloc::format!("{value:e}");
    let (mantissa, exponent) = rendered.split_once('e').unwrap_or((rendered.as_str(), "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().unwrap_or_default();
    // Trailing zeros are not part of the shortest form; `{:e}` does not emit
    // them, but a `1e3` style input would leave one behind.
    let trimmed = digits.trim_end_matches('0');
    if trimmed.is_empty() {
        return (String::from("0"), 0);
    }
    (trimmed.to_string(), exponent)
}

/// Arranges digits and an exponent into ECMA-262's notation.
///
/// `point` is the specification's `n`: the position of the decimal point
/// relative to the start of the digits, so the value is `0.digits × 10^point`.
fn spell(digits: &str, exponent: i32) -> String {
    let count = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let point = exponent + 1;

    if point > PLAIN_UPPER || point <= PLAIN_LOWER {
        return exponential(digits, point);
    }
    if count <= point {
        // Whole number: every digit is an integer digit, and the rest are
        // zeros the shortest form did not need to carry.
        let mut text = String::from(digits);
        for _ in 0..(point - count) {
            text.push('0');
        }
        return text;
    }
    if point > 0 {
        // The point falls inside the digits.
        let mut text = String::with_capacity(digits.len() + 1);
        let split = usize::try_from(point).unwrap_or(0);
        text.push_str(&digits[..split]);
        text.push('.');
        text.push_str(&digits[split..]);
        return text;
    }
    // The point falls before them, so leading zeros carry the magnitude.
    let mut text = String::from("0.");
    for _ in 0..(-point) {
        text.push('0');
    }
    text.push_str(digits);
    text
}

/// Parses a token the way JavaScript `parseFloat` does.
///
/// Like `parseInt`, this takes a *prefix* and ignores whatever follows, which
/// is why `"1.2.3"` is `1.2` and `"1,2"` is `1`. Unlike `parseInt` it has no
/// radix, accepts an exponent, and accepts the word `Infinity`. Anything with
/// no valid prefix at all is `NaN`, and so is the empty string — which matters
/// because splitting on a delimiter routinely produces one.
///
/// Rust's own `str::parse::<f64>` is not a substitute: it requires the *whole*
/// string to be a number and accepts spellings JavaScript does not, such as
/// `inf` and `2.5e`.
#[must_use]
pub(crate) fn parse_float(token: &str) -> f64 {
    let trimmed = token.trim_start_matches(super::delim::is_js_whitespace);
    let negative = trimmed.starts_with('-');
    // The sign is consumed here and re-attached at the end, so every length
    // below is measured against one string.
    let body = if negative || trimmed.starts_with('+') {
        &trimmed[1..]
    } else {
        trimmed
    };

    if body.starts_with("Infinity") {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    let integer = digits(body);
    let mut length = integer;
    let mut fraction = 0;
    if body[length..].starts_with('.') {
        fraction = 1 + digits(&body[length + 1..]);
        length += fraction;
    }
    // A bare `.` with digits on neither side is not a number, and neither is
    // an empty token — the common case, because splitting on a delimiter
    // routinely leaves a gap.
    if integer == 0 && fraction <= 1 {
        return f64::NAN;
    }

    if body[length..].starts_with(['e', 'E']) {
        let after = &body[length + 1..];
        let signs = usize::from(after.starts_with(['+', '-']));
        let count = digits(&after[signs..]);
        // An exponent marker with no digits behind it is not part of the
        // number; the prefix simply ends before it.
        if count > 0 {
            length += 1 + signs + count;
        }
    }

    let parsed = body[..length].parse::<f64>().unwrap_or(f64::NAN);
    if negative { -parsed } else { parsed }
}

/// How many ASCII digits start this string.
fn digits(text: &str) -> usize {
    text.bytes().take_while(u8::is_ascii_digit).count()
}

/// `d.dddde±xx`, with the sign always written and the exponent unpadded.
fn exponential(digits: &str, point: i32) -> String {
    let mut text = String::with_capacity(digits.len() + 6);
    let mut characters = digits.chars();
    if let Some(first) = characters.next() {
        text.push(first);
    }
    let rest = characters.as_str();
    if !rest.is_empty() {
        text.push('.');
        text.push_str(rest);
    }
    text.push('e');
    let power = point - 1;
    if power >= 0 {
        text.push('+');
    }
    text.push_str(&power.to_string());
    text
}
