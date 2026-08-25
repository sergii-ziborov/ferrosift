//! IEEE-754 conversion, as the reference's `ieee754` package performs it.

use alloc::string::String;
use alloc::vec::Vec;

use crate::jscompat::double;

/// Which width the operation is working in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Width {
    /// Four bytes.
    Single,
    /// Eight bytes.
    Double,
}

impl Width {
    /// How many bytes one value occupies.
    pub const fn size(self) -> usize {
        match self {
            Self::Single => 4,
            Self::Double => 8,
        }
    }
}

/// Packs parsed numbers into bytes.
#[must_use]
pub fn encode(values: &[f64], width: Width, little_endian: bool) -> Vec<u8> {
    let mut output = Vec::with_capacity(values.len() * width.size());
    for value in values {
        let bits = pack(*value, width);
        let bytes = bits.to_be_bytes();
        // Only the low bytes belong to a single; the rest are the padding of
        // the u64 the packer works in.
        push(&mut output, &bytes[8 - width.size()..], little_endian);
    }
    output
}

/// Packs one number the way the reference's `ieee754` package does.
///
/// Deliberately not `value as f32`. That rounds to nearest with ties to even;
/// the reference adds a rounding term of just under half an ulp and then
/// *truncates*, which sends an exact tie the other way. `16777217` is the
/// smallest case that shows it: `as f32` gives `4b800000` and the reference
/// gives `4b800001`, and a port that used the cast would be right on every
/// value except the ones that land exactly between two singles.
///
/// The arithmetic below is the package's, transliterated. Its `Math.log` and
/// `Math.pow` are replaced by exact bit-level equivalents rather than by an
/// approximation — partly because this crate is `no_std` and has no `libm`,
/// and partly because the package's own two correction steps converge on the
/// exponent this computes directly.
fn pack(value: f64, width: Width) -> u64 {
    let mantissa_bits = match width {
        Width::Single => 23i32,
        Width::Double => 52,
    };
    let total_bits = i32::try_from(width.size()).unwrap_or(8) * 8;
    let exponent_max = (1i64 << (total_bits - mantissa_bits - 1)) - 1;
    let bias = exponent_max >> 1;

    // The rounding term, and the reason a tie rounds away from zero. It is a
    // hair under half an ulp, which is what stops a value already on a
    // representable single from being pushed up to the next one.
    let round_to = if mantissa_bits == 23 {
        exp2(-24) - exp2(-77)
    } else {
        0.0
    };

    let sign = u64::from(value < 0.0 || (value == 0.0 && value.is_sign_negative()));
    let mut magnitude = f64::from_bits(value.to_bits() & !(1u64 << 63));

    let (mantissa, exponent) = if magnitude.is_nan() || magnitude.is_infinite() {
        // A NaN becomes a mantissa of exactly one, which is why it packs as
        // `7f800001` rather than as any quiet pattern.
        (if magnitude.is_nan() { 1.0 } else { 0.0 }, exponent_max)
    } else if magnitude == 0.0 {
        (0.0, 0)
    } else {
        let mut power = i64::from(floor_log2(magnitude));
        let mut scale = exp2_i64(-power);
        if magnitude * scale < 1.0 {
            power -= 1;
            scale *= 2.0;
        }
        if power + bias >= 1 {
            magnitude += round_to / scale;
        } else {
            magnitude += round_to * exp2_i64(1 - bias);
        }
        if magnitude * scale >= 2.0 {
            power += 1;
            scale /= 2.0;
        }

        if power + bias >= exponent_max {
            (0.0, exponent_max)
        } else if power + bias >= 1 {
            (
                ((magnitude * scale) - 1.0) * exp2(mantissa_bits),
                power + bias,
            )
        } else {
            (magnitude * exp2_i64(bias - 1) * exp2(mantissa_bits), 0)
        }
    };

    // The package writes the mantissa out byte by byte through `& 0xff`, which
    // truncates toward zero; dividing by 256 between bytes is exact, so the
    // bytes written are those of the truncated integer.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the mantissa is non-negative and below 2^mantissa_bits here"
    )]
    let mantissa = mantissa as u64;
    #[expect(clippy::cast_sign_loss, reason = "the exponent is non-negative here")]
    let exponent = exponent as u64;
    (sign << (total_bits - 1)) | (exponent << mantissa_bits) | mantissa
}

/// Exactly `floor(log2(value))`, for a positive finite value.
fn floor_log2(value: f64) -> i32 {
    let bits = value.to_bits();
    let raw = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    if raw > 0 {
        return raw - 1023;
    }
    // Subnormal: the exponent field says nothing, so the highest set mantissa
    // bit does.
    let mantissa = bits & ((1u64 << 52) - 1);
    let highest = 63 - i32::try_from(mantissa.leading_zeros()).unwrap_or(0);
    highest - 1074
}

/// Exactly `2^power`, built from the exponent field rather than computed.
fn exp2(power: i32) -> f64 {
    if power > 1023 {
        return f64::INFINITY;
    }
    if power >= -1022 {
        let biased = u64::try_from(power + 1023).unwrap_or(0);
        return f64::from_bits(biased << 52);
    }
    if power >= -1074 {
        let shift = u32::try_from(power + 1074).unwrap_or(0);
        return f64::from_bits(1u64 << shift);
    }
    0.0
}

/// [`exp2`] over the wider type the exponent arithmetic uses.
fn exp2_i64(power: i64) -> f64 {
    exp2(i32::try_from(power).unwrap_or(if power < 0 { i32::MIN } else { i32::MAX }))
}

/// Appends one value's bytes in the requested order.
fn push(output: &mut Vec<u8>, big_endian: &[u8], little_endian: bool) {
    if little_endian {
        output.extend(big_endian.iter().rev());
    } else {
        output.extend_from_slice(big_endian);
    }
}

/// Unpacks bytes into numbers.
///
/// The caller has already checked the length is a whole number of values, so a
/// trailing partial value cannot reach here.
#[must_use]
pub fn decode(input: &[u8], width: Width, little_endian: bool) -> Vec<f64> {
    input
        .chunks_exact(width.size())
        .map(|chunk| {
            let mut ordered: Vec<u8> = chunk.to_vec();
            if little_endian {
                ordered.reverse();
            }
            match width {
                Width::Single => {
                    let bits = u32::from_be_bytes([ordered[0], ordered[1], ordered[2], ordered[3]]);
                    // Widened, not reinterpreted: the reference reads a single
                    // and hands back a JavaScript number, which is a double.
                    f64::from(f32::from_bits(bits))
                }
                Width::Double => {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&ordered);
                    f64::from_bits(u64::from_be_bytes(bytes))
                }
            }
        })
        .collect()
}

/// Splits a delimited string into numbers, `parseFloat` style.
///
/// Every field becomes a value, including empty ones: a gap left by two
/// adjacent delimiters parses as `NaN` and is written out as a NaN pattern
/// rather than skipped.
///
/// The delimiter is never empty: this operation's list does not offer
/// `Nothing (separate chars)`, so the code-unit splitting that option would
/// need does not arise here.
#[must_use]
pub fn parse_all(input: &str, delimiter: &str) -> Vec<f64> {
    input.split(delimiter).map(double::parse_float).collect()
}

/// Renders numbers back into a delimited string.
#[must_use]
pub fn render_all(values: &[f64], delimiter: &str) -> String {
    let mut output = String::new();
    for (position, value) in values.iter().enumerate() {
        if position > 0 {
            output.push_str(delimiter);
        }
        output.push_str(&double::format(*value));
    }
    output
}
