//! Rendering a float the way `Number.prototype.toString` does.
//!
//! Rust's `Display` for `f64` already produces the shortest decimal that reads
//! back as the same value, which is the same rule JavaScript uses, so the two
//! agree across the range where both write plain decimals. They part at the
//! ends: JavaScript switches to exponential notation at 1e21 and below 1e-6,
//! and spells the exponent with a sign it never omits.

use alloc::format;
use alloc::string::{String, ToString};

/// The magnitude at which the reference stops writing plain decimals.
const UPPER: f64 = 1e21;

/// The magnitude below which it does the same at the other end.
const LOWER: f64 = 1e-6;

/// Formats `value` as the reference would print it.
pub(crate) fn to_js_string(value: f64) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value.is_infinite() {
        return String::from(if value > 0.0 { "Infinity" } else { "-Infinity" });
    }
    // Covers negative zero, which prints without its sign.
    if value == 0.0 {
        return String::from("0");
    }

    let magnitude = if value < 0.0 { -value } else { value };
    if !(LOWER..UPPER).contains(&magnitude) {
        return exponential(value);
    }
    value.to_string()
}

/// Rounds the way `Math.round` does, which is not the way `f64::round` does.
///
/// Two reasons this is written out rather than called. `f64::round` lives in
/// `std`, and this crate is `no_std`, so it is not there to call. And it
/// rounds half *away from zero* while the reference rounds half *up*: at
/// -0.5 one answers -1 and the other 0. Colour channels stay positive, where
/// the two agree, but a rule that happens to agree on today's inputs is not
/// the rule the reference has.
pub(crate) fn js_round(value: f64) -> f64 {
    if value.is_nan() || value.is_infinite() {
        return value;
    }
    floor(value + 0.5)
}

/// The largest integer not above `value`, without `std`.
///
/// Anything at or beyond 2^53 is already an integer in this format, so the
/// conversion below is only asked about values it can hold exactly.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "values at or beyond 2^53 return above, so both casts only see ones they hold exactly"
)]
fn floor(value: f64) -> f64 {
    const INTEGRAL: f64 = 9_007_199_254_740_992.0;
    if value >= INTEGRAL || value <= -INTEGRAL {
        return value;
    }
    // The cast truncates toward zero, so a negative with a fraction lands one
    // above the floor and needs stepping down.
    let truncated = value as i64 as f64;
    if truncated > value {
        truncated - 1.0
    } else {
        truncated
    }
}

/// Exponential form, with the explicit `+` the reference always writes.
fn exponential(value: f64) -> String {
    let rendered = format!("{value:e}");
    match rendered.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => rendered,
    }
}
