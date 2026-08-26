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
