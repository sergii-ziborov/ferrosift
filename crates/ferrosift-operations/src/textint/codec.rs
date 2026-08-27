use alloc::string::String;
use alloc::vec::Vec;

use num_bigint::BigInt;
use num_traits::{Signed, Zero};

/// What the input turned out to be.
pub(crate) enum Reading {
    /// A number, in whichever base it was written.
    Number(BigInt),
    /// Text, whose characters become the digits of a number.
    Text,
}

/// Decides what the trimmed input is, the way the reference decides.
///
/// Order matters and is the reference's. Digits are read as a number before
/// anything considers them text, so `123` is one hundred and twenty-three and
/// `"123"` -- the same digits in quotes -- is three characters. A caller who
/// wants the digits themselves has to quote them.
pub(crate) fn classify(trimmed: &str) -> Reading {
    if trimmed.is_empty() {
        return Reading::Number(BigInt::zero());
    }
    if let Some(digits) = hexadecimal(trimmed) {
        return Reading::Number(digits);
    }
    if let Some(digits) = decimal(trimmed) {
        return Reading::Number(digits);
    }
    Reading::Text
}

/// `0x` and at least one hexadecimal digit, in either case.
fn hexadecimal(text: &str) -> Option<BigInt> {
    let rest = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    if rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    BigInt::parse_bytes(rest.as_bytes(), 16)
}

/// Digits, with an optional leading sign.
fn decimal(text: &str) -> Option<BigInt> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    BigInt::parse_bytes(text.as_bytes(), 10)
}

/// Strips a matching pair of quotes, if the text is wrapped in one.
///
/// The reference tests only that the first and last characters are quotes, not
/// that they are the *same* quote -- so `'text"` is treated as quoted, and the
/// two odd characters are removed. Reproduced rather than tightened.
pub(crate) fn unquote(text: &str) -> &str {
    let mut characters = text.chars();
    let first = characters.next();
    let last = characters.next_back();
    match (first, last) {
        (Some('"' | '\''), Some('"' | '\'')) => {
            let start = text.len() - text[1..].len();
            let end = text.len() - last.map_or(0, char::len_utf8);
            &text[start..end]
        }
        _ => text,
    }
}

/// Reads text as the digits of a base-256 number, or refuses a character that
/// has no place in one.
///
/// Every character must fit in a byte. The reference walks UTF-16 code units
/// and refuses any above 255, which means a character outside the Latin-1
/// range is refused whether it is one code unit or two -- so refusing on the
/// character itself agrees with it.
pub(crate) fn text_to_number(text: &str) -> Option<BigInt> {
    let mut bytes = Vec::with_capacity(text.len());
    for character in text.chars() {
        let code = u32::from(character);
        if code > 255 {
            return None;
        }
        bytes.push(u8::try_from(code).ok()?);
    }
    if bytes.is_empty() {
        return Some(BigInt::zero());
    }
    Some(BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes))
}

/// Writes a number back as the characters its bytes stand for.
///
/// A negative number answers nothing at all: the reference's loop runs while
/// the value is above zero, and a negative one never enters it. Zero answers
/// nothing for the same reason. Both are quirks of the loop rather than
/// decisions, and both are reproduced.
pub(crate) fn number_to_text(value: &BigInt) -> String {
    if !value.is_positive() {
        return String::new();
    }
    let (_, bytes) = value.to_bytes_be();
    bytes.into_iter().map(char::from).collect()
}

/// Writes a number in hexadecimal, the way the reference does.
///
/// A negative number keeps its sign *inside* the prefix -- `0x-5` -- because
/// the reference concatenates `"0x"` with `toString(16)`, and that rendering
/// carries the minus. It is not a valid literal and it is what the reference
/// produces.
pub(crate) fn to_hexadecimal(value: &BigInt) -> String {
    let mut output = String::from("0x");
    output.push_str(&value.to_str_radix(16));
    output
}
