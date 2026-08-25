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
            .min(SATURATION);
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
