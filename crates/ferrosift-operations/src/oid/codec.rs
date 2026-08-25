//! ASN.1 object identifiers, as the reference's ASN.1 library encodes them.

use alloc::string::String;
use alloc::vec::Vec;

use core::fmt::Write as _;

use ferrosift_core::OperationError;
use num_bigint::BigUint;
use num_traits::{FromPrimitive, ToPrimitive, Zero};

use crate::failure::failed;

/// How many arcs the first byte packs together.
const FIRST_PAIR_SCALE: u32 = 40;

/// Encodes a dotted object identifier as hexadecimal.
///
/// Two rules, and they are not the same rule. The first two arcs are combined
/// into `a * 40 + b` and written as plain hexadecimal — *not* base-128, and not
/// padded beyond two digits. Every later arc is written base-128 with a
/// continuation bit on all but its final byte.
///
/// That asymmetry is the reference's, and it is a real bug for a first pair
/// above 255: `2.999` becomes `437`, three hex digits, which no ASN.1 decoder
/// will read back. It is reproduced because the operation's whole purpose is to
/// say what this reference produces. The inverse direction cannot undo it, and
/// `Hex to Object Identifier` says so in its own documentation.
///
/// # Errors
///
/// Returns an error when the identifier is malformed, or when it has fewer than
/// two arcs — see the module documentation for why the second is a divergence.
pub fn to_hex(input: &str) -> Result<String, OperationError> {
    if input.is_empty()
        || !input
            .chars()
            .all(|glyph| glyph.is_ascii_digit() || glyph == '.')
    {
        return Err(failed("asn1.oid.malformed"));
    }

    let arcs: Vec<&str> = input.split('.').collect();
    let (Some(first), Some(second)) = (arcs.first(), arcs.get(1)) else {
        return Err(failed("asn1.oid.too_few_arcs"));
    };
    // An empty arc parses as nothing at all here, where a *later* empty arc is
    // read as zero. The reference splits on the same difference, so the two are
    // handled separately rather than unified.
    let (Some(first), Some(second)) = (parse_double(first), parse_double(second)) else {
        return Err(failed("asn1.oid.too_few_arcs"));
    };

    // Double arithmetic, deliberately. The reference combines the first pair
    // with `parseInt` and JavaScript's `*` and `+`, which are IEEE-754
    // doubles, so a first arc past 2^53 loses precision before it is ever
    // written: `9007199254740993.1` and `9007199254740992.0` produce the same
    // bytes. Later arcs go through an exact big integer instead, so only this
    // pair rounds. Computing this pair exactly would be arithmetically better
    // and would disagree with every identifier the reference has written.
    // Not `mul_add`, which fuses to a single rounding where JavaScript rounds
    // twice. The two agree on every small identifier and part company exactly
    // where this matters.
    let combined = first * f64::from(FIRST_PAIR_SCALE) + second;
    let Some(combined) = BigUint::from_f64(combined) else {
        // Negative, infinite, or not a number: the reference carries each of
        // those into its formatter and prints the word. See the module
        // documentation for why this refuses.
        return Err(failed("asn1.oid.not_a_number"));
    };
    let mut output = format_byte(&combined);
    for arc in arcs.iter().skip(2) {
        // A missing arc is zero here, which is how `1.2.` gains a trailing
        // `00`.
        let value = parse_arc(arc).unwrap_or_else(BigUint::zero);
        output.push_str(&base128(&value));
    }
    Ok(output)
}

/// Reads one arc exactly, or `None` when it has no digits at all.
fn parse_arc(arc: &str) -> Option<BigUint> {
    if arc.is_empty() {
        return None;
    }
    // The characters were checked to be digits before the split.
    arc.parse::<BigUint>().ok()
}

/// Reads one arc as a double, the way the first pair is read.
///
/// Both this and [`parse_arc`] see the same digits and can disagree about
/// them, which is the reference's arrangement rather than an oversight here.
fn parse_double(arc: &str) -> Option<f64> {
    if arc.is_empty() {
        return None;
    }
    arc.parse::<f64>().ok()
}

/// Renders a value as hex, padded to two digits only when it needs one.
///
/// Deliberately not padded to a whole number of bytes. A value above 255 comes
/// out as three or more digits, which is what the reference does and what makes
/// the first pair unreadable above that point.
fn format_byte(value: &BigUint) -> String {
    let hex = value.to_str_radix(16);
    if hex.len() == 1 {
        let mut padded = String::from("0");
        padded.push_str(&hex);
        return padded;
    }
    hex
}

/// Renders a value base-128, continuation bit set on all but the last byte.
fn base128(value: &BigUint) -> String {
    let mut groups: Vec<u8> = Vec::new();
    let mut rest = value.clone();
    let base = BigUint::from(128u8);
    loop {
        let digit = (&rest % &base).to_u8().unwrap_or_default();
        groups.push(digit);
        rest /= &base;
        if rest.is_zero() {
            break;
        }
    }

    let mut output = String::with_capacity(groups.len() * 2);
    for (position, group) in groups.iter().rev().enumerate() {
        let last = position + 1 == groups.len();
        let byte = if last { *group } else { group | 0x80 };
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Decodes hexadecimal into a dotted object identifier.
///
/// The first byte is split as `floor(b / 40)` and `b % 40`, which is the
/// inverse of the encoder only while the first pair fits in one byte. Above
/// that the encoder emits more than one byte without a continuation bit, and
/// this reads the first of them alone — the round trip is broken at the source,
/// not here.
///
/// A trailing group whose continuation bit is never cleared is dropped rather
/// than reported, which is the reference's behaviour and makes truncated input
/// decode to its complete prefix.
///
/// # Errors
///
/// Returns an error when a two-character group is not hexadecimal — see the
/// module documentation for why that is a divergence.
pub fn from_hex(input: &str) -> Result<String, OperationError> {
    let compact: String = input
        .chars()
        .filter(|glyph| !glyph.is_whitespace())
        .collect();
    let digits: Vec<char> = compact.chars().collect();

    let first = group(&digits, 0)?;
    let mut output = String::new();
    let _ = write!(
        output,
        "{}.{}",
        u32::from(first) / FIRST_PAIR_SCALE,
        u32::from(first) % FIRST_PAIR_SCALE
    );

    // An unterminated group contributes nothing, so `pending` is deliberately
    // dropped when the input runs out mid-arc.
    let mut pending = BigUint::ZERO;
    let mut at = 2;
    while at < digits.len() {
        let byte = group(&digits, at)?;
        pending = pending * BigUint::from(128u8) + BigUint::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            let _ = write!(output, ".{}", pending.to_str_radix(10));
            pending = BigUint::ZERO;
        }
        at += 2;
    }

    Ok(output)
}

/// Reads the two hex characters at `at`, or the single trailing one.
///
/// The reference reads each pair with `parseInt`, which takes the longest
/// valid prefix and stops: `0z` is zero, not an error. That prefix rule is
/// reproduced. What `parseInt` returns when there is no prefix at all is
/// `NaN`, and the reference carries that forward into its bignum — see the
/// module documentation for why this refuses instead.
fn group(digits: &[char], at: usize) -> Result<u8, OperationError> {
    let chunk: String = digits.iter().skip(at).take(2).collect();
    if chunk.is_empty() {
        return Err(failed("asn1.oid.not_hex"));
    }
    match crate::jscompat::number::parse(&chunk, 16) {
        crate::jscompat::number::JsInt::Value(value) => {
            u8::try_from(value).map_err(|_| failed("asn1.oid.not_hex"))
        }
        crate::jscompat::number::JsInt::Nan => Err(failed("asn1.oid.not_hex")),
    }
}
