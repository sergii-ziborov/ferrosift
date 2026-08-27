use crate::jscompat::delim::is_js_whitespace;

/// Result of the ECMAScript `parseInt` prefix-parsing algorithm.
///
/// Values far outside the byte range saturate: callers only distinguish
/// "not a number", "negative", and "0..=255 or too large", exactly the split
/// the pinned `CyberChef` byte-array validation applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JsInt {
    /// The token has no parsable digits (`parseInt` returned `NaN`).
    Nan,
    /// The parsed integer value, saturated to `±1_000_000`.
    Value(i64),
}

const SATURATION: i64 = 1_000_000;

/// Parses a token the way JavaScript `parseInt(token, radix)` does: leading
/// whitespace is skipped, one optional sign is honored, and the longest
/// prefix of valid digits wins while trailing garbage is ignored.
///
/// At radix sixteen a leading `0x` or `0X` is consumed rather than parsed, so
/// `parseInt("0x1f", 16)` is thirty-one and `parseInt("0x", 16)` is `NaN` —
/// the prefix is stripped and nothing is left. At any other radix those are
/// ordinary characters, so `parseInt("0x1f", 10)` is zero.
///
/// That branch was missing until `tests/jscompat.rs` compared this function
/// against Node directly; every operation reading a hex token shared the
/// mistake, and none of their own corpus cases had reached it.
pub(crate) fn parse(token: &str, radix: u32) -> JsInt {
    scan(token, radix, SATURATION)
}

/// The same reading, without the byte-range ceiling.
///
/// [`parse`] stops counting at a million because every caller it was written
/// for only had to tell a byte from something out of range. An address does
/// not fit that: four thousand million is an ordinary IPv4 address written as
/// a decimal, and saturating it turned `3232235521` into `1000000` and
/// `192.168.0.1` into `0.15.66.64`.
///
/// The ceiling here is what a JavaScript number can hold exactly, which is the
/// real limit the reference is working under.
pub(crate) fn parse_wide(token: &str, radix: u32) -> JsInt {
    scan(token, radix, MAX_SAFE_INTEGER)
}

/// What a JavaScript number counts by ones up to.
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// The same reading once more, as the *number* the reference ends up holding.
///
/// `parseInt` produces a `Number`, not an integer. Both readings above stop at
/// a ceiling, which is right for a caller that goes on to index with the answer
/// and wrong for one that goes on to print it: a long enough run of digits
/// rounds to a double there and prints in exponential form, where saturating
/// would print a round number the reference never had.
///
/// Radix ten only, and that restriction is what makes it exact. Rust's
/// decimal-to-double conversion is correctly rounded and so is the reference's,
/// so handing the digits over unchanged gives the same double; reproducing that
/// rounding for base thirty-six would mean writing it out by hand, and no
/// caller wants it.
pub(crate) fn parse_decimal(token: &str) -> f64 {
    let trimmed = token.trim_start_matches(is_js_whitespace);
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits = rest.len()
        - rest
            .trim_start_matches(|value: char| value.is_ascii_digit())
            .len();
    if digits == 0 {
        return f64::NAN;
    }
    // Digits only, so this cannot fail; a run too long to hold reads as
    // infinity, which is what the reference gets for it too.
    let value = rest[..digits].parse::<f64>().unwrap_or(f64::NAN);
    if negative { -value } else { value }
}

/// The reading both entry points share, differing only in where they stop.
fn scan(token: &str, radix: u32, ceiling: i64) -> JsInt {
    let mut chars = token.chars().skip_while(|value| is_js_whitespace(*value));
    let mut first = chars.next();
    let negative = match first {
        Some('-') => {
            first = chars.next();
            true
        }
        Some('+') => {
            first = chars.next();
            false
        }
        _ => false,
    };

    if radix == 16 && first == Some('0') {
        let mut lookahead = chars.clone();
        if matches!(lookahead.next(), Some('x' | 'X')) {
            chars = lookahead;
            first = chars.next();
        }
    }

    let mut value: i64 = 0;
    let mut digits = 0_usize;
    while let Some(digit) = first.and_then(|symbol| symbol.to_digit(radix)) {
        value = value
            .saturating_mul(i64::from(radix))
            .saturating_add(i64::from(digit))
            .min(ceiling);
        digits += 1;
        first = chars.next();
    }

    if digits == 0 {
        JsInt::Nan
    } else if negative {
        JsInt::Value(-value)
    } else {
        JsInt::Value(value)
    }
}

/// Converts a parsed token into the byte the pinned `CyberChef` node API
/// produces: `NaN` coerces to zero and integers must already be in range.
pub(crate) fn to_byte(parsed: JsInt) -> Option<u8> {
    match parsed {
        JsInt::Nan => Some(0),
        JsInt::Value(value) => u8::try_from(value).ok(),
    }
}
