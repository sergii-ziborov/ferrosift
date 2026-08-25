//! Punycode, RFC 3492, and the domain-name wrapper around it.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const BASE: u32 = 36;
const TMIN: u32 = 1;
const TMAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const INITIAL_BIAS: u32 = 72;
const INITIAL_N: u32 = 128;
const DELIMITER: char = '-';

/// Encodes one label: the bare RFC 3492 transform, with no `xn--` prefix.
pub(super) fn encode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let points: Vec<u32> = input.chars().map(u32::from).collect();
    let mut output: String = points
        .iter()
        .filter(|point| **point < 0x80)
        .filter_map(|point| char::from_u32(*point))
        .collect();

    let basic_length = output.chars().count();
    let mut handled = basic_length;
    if basic_length > 0 {
        output.push(DELIMITER);
    }

    let mut n = INITIAL_N;
    let mut delta: u32 = 0;
    let mut bias = INITIAL_BIAS;

    while handled < points.len() {
        context.ensure_active()?;
        // The next code point to deal with is the smallest one not yet
        // handled. Working in ascending order is what lets the deltas stay
        // small enough to encode compactly.
        let m = points
            .iter()
            .copied()
            .filter(|point| *point >= n)
            .min()
            .ok_or_else(overflow)?;

        let advance = m
            .checked_sub(n)
            .and_then(|gap| gap.checked_mul(u32::try_from(handled + 1).ok()?))
            .ok_or_else(overflow)?;
        delta = delta.checked_add(advance).ok_or_else(overflow)?;
        n = m;

        for point in &points {
            if *point < n {
                delta = delta.checked_add(1).ok_or_else(overflow)?;
            }
            if *point != n {
                continue;
            }
            let mut q = delta;
            let mut k = BASE;
            loop {
                let t = threshold(k, bias);
                if q < t {
                    break;
                }
                let digit = t + (q - t) % (BASE - t);
                output.push(digit_to_basic(digit));
                q = (q - t) / (BASE - t);
                k += BASE;
            }
            output.push(digit_to_basic(q));
            bias = adapt(delta, u32::try_from(handled + 1).map_err(|_| overflow())?, handled == basic_length);
            delta = 0;
            handled += 1;
        }

        delta = delta.checked_add(1).ok_or_else(overflow)?;
        n = n.checked_add(1).ok_or_else(overflow)?;
    }

    Ok(output)
}

/// Decodes one label: the bare RFC 3492 transform, with no `xn--` handling.
pub(super) fn decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let characters: Vec<char> = input.chars().collect();
    // Everything before the last hyphen is literal. Using the last one rather
    // than the first is what allows a literal hyphen inside the basic part.
    let split = characters.iter().rposition(|c| *c == DELIMITER).unwrap_or(0);

    let mut output: Vec<u32> = Vec::with_capacity(characters.len());
    for character in &characters[..split] {
        if !character.is_ascii() {
            return Err(failed("encoding.punycode.not_basic"));
        }
        output.push(u32::from(*character));
    }

    let mut cursor = if split > 0 { split + 1 } else { 0 };
    let mut n = INITIAL_N;
    let mut i: u32 = 0;
    let mut bias = INITIAL_BIAS;

    while cursor < characters.len() {
        context.ensure_active()?;
        let previous = i;
        let mut weight: u32 = 1;
        let mut k = BASE;
        loop {
            let character = *characters.get(cursor).ok_or_else(|| failed("encoding.punycode.truncated"))?;
            cursor += 1;
            let digit = basic_to_digit(character)
                .ok_or_else(|| failed("encoding.punycode.invalid_digit"))?;
            let step = digit.checked_mul(weight).ok_or_else(overflow)?;
            i = i.checked_add(step).ok_or_else(overflow)?;
            let t = threshold(k, bias);
            if digit < t {
                break;
            }
            weight = weight.checked_mul(BASE - t).ok_or_else(overflow)?;
            k += BASE;
        }

        let length = u32::try_from(output.len() + 1).map_err(|_| overflow())?;
        bias = adapt(i - previous, length, previous == 0);
        n = n.checked_add(i / length).ok_or_else(overflow)?;
        i %= length;

        let position = usize::try_from(i).map_err(|_| overflow())?;
        output.insert(position, n);
        i += 1;
    }

    output
        .into_iter()
        .map(|point| char::from_u32(point).ok_or_else(|| failed("encoding.punycode.not_a_character")))
        .collect()
}

/// Encodes a domain name, prefixing each non-ASCII label with `xn--`.
pub(super) fn to_ascii(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    map_domain(input, context, |label, context| {
        if label.chars().all(|c| c <= '\u{7e}') {
            Ok(String::from(label))
        } else {
            let mut encoded = String::from("xn--");
            encoded.push_str(&encode(label, context)?);
            Ok(encoded)
        }
    })
}

/// Decodes a domain name, unwrapping each `xn--` label.
pub(super) fn to_unicode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    map_domain(input, context, |label, context| {
        if let Some(rest) = label.strip_prefix("xn--") {
            // The reference lower-cases before decoding, so `XN--` labels and
            // mixed-case payloads both decode rather than being rejected.
            let lowered: String = rest.chars().flat_map(char::to_lowercase).collect();
            decode(&lowered, context)
        } else {
            Ok(String::from(label))
        }
    })
}

/// Applies a per-label transform across a domain, keeping any mailbox prefix.
///
/// The `@` split is not decoration: the reference accepts `user@münchen.de`
/// and transforms only the part after the mailbox. Four characters count as a
/// label separator on the way in, and all four are normalised to `.` on the
/// way out, so the output separator is never the one that was typed.
///
/// A second `@` discards everything after it. The reference splits on every
/// `@` and then reads only the first two pieces, so `a@b@c` becomes `a@b` --
/// data loss rather than an error. Keeping the tail would be the more sensible
/// behaviour and the wrong answer.
fn map_domain(
    input: &str,
    context: &OperationContext<'_>,
    transform: impl Fn(&str, &OperationContext<'_>) -> Result<String, OperationError>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut pieces = input.split('@');
    let first = pieces.next().unwrap_or("");
    let (prefix, domain) = match pieces.next() {
        Some(rest) => (Some(first), rest),
        None => (None, first),
    };

    let mut output = String::new();
    if let Some(mailbox) = prefix {
        output.push_str(mailbox);
        output.push('@');
    }

    let normalised: String = domain
        .chars()
        .map(|c| if is_separator(c) { '.' } else { c })
        .collect();

    let mut first = true;
    for label in normalised.split('.') {
        if !first {
            output.push('.');
        }
        first = false;
        output.push_str(&transform(label, context)?);
    }
    Ok(output)
}

/// The four characters the reference treats as a label separator.
fn is_separator(character: char) -> bool {
    matches!(character, '.' | '\u{3002}' | '\u{ff0e}' | '\u{ff61}')
}

/// The digit threshold for one round, clamped to `tmin..=tmax`.
fn threshold(k: u32, bias: u32) -> u32 {
    if k <= bias {
        TMIN
    } else if k >= bias + TMAX {
        TMAX
    } else {
        k - bias
    }
}

/// Retunes the bias after a code point, so later deltas encode compactly.
fn adapt(delta: u32, points: u32, first: bool) -> u32 {
    let mut delta = if first { delta / DAMP } else { delta / 2 };
    delta += delta / points;
    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + (((BASE - TMIN + 1) * delta) / (delta + SKEW))
}

/// Maps 0-25 to `a`-`z` and 26-35 to `0`-`9`.
fn digit_to_basic(digit: u32) -> char {
    if digit < 26 {
        char::from(b'a' + u8::try_from(digit).unwrap_or(0))
    } else {
        char::from(b'0' + u8::try_from(digit - 26).unwrap_or(0))
    }
}

/// The inverse of `digit_to_basic`, case-insensitive on letters.
fn basic_to_digit(character: char) -> Option<u32> {
    match character {
        'a'..='z' => Some(u32::from(character) - u32::from('a')),
        'A'..='Z' => Some(u32::from(character) - u32::from('A')),
        '0'..='9' => Some(u32::from(character) - u32::from('0') + 26),
        _ => None,
    }
}

/// The single overflow failure both directions raise.
fn overflow() -> OperationError {
    failed("encoding.punycode.overflow")
}
