//! What the filetime conversions refuse, and where they do not round-trip.
//!
//! Two things the corpus cannot hold. The first is a refusal: these
//! operations call the reference's constructor directly rather than through a
//! dish, so text it cannot read *stops the recipe* instead of becoming
//! not-a-number. That difference is invisible in any comparison of answers.
//!
//! The second is the endianness swap, which is transcribed rather than tidied.
//! The two directions are not inverses for an odd-length string: one appends
//! `0` and the first character, the other moves the last character to the
//! front. A cleaned-up implementation would round-trip and disagree with the
//! reference, so the disagreement is stated here rather than left to be
//! discovered.

#![cfg(feature = "bignum")]

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

const SECONDS: &str = "Seconds (s)";
const MICROSECONDS: &str = "Microseconds (\u{03bc}s)";

fn convert(operation: &str, units: &str, format: &str, input: &str) -> Result<String, ()> {
    let arguments = Arguments::from([
        ("units".into(), ArgumentValue::Text(units.into())),
        ("format".into(), ArgumentValue::Text(format.into())),
    ]);
    match support::run_with_budget(
        operation,
        arguments,
        support::text(input),
        support::budget(),
    ) {
        Ok(result) => match result.value {
            Value::Text(value) => Ok(value.text),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

fn to_filetime(units: &str, format: &str, input: &str) -> Result<String, ()> {
    convert("time.filetime.encode@1", units, format, input)
}

fn to_unix(units: &str, format: &str, input: &str) -> Result<String, ()> {
    convert("time.filetime.decode@1", units, format, input)
}

#[test]
fn text_that_is_not_a_number_stops_the_recipe() {
    // A dish would have substituted not-a-number here and carried on. These
    // operations construct the number themselves, so they see the exception.
    assert!(to_filetime(SECONDS, "Decimal", "apples").is_err());
    assert!(to_unix(SECONDS, "Decimal", "apples").is_err());
    assert!(to_unix(SECONDS, "Hex (big endian)", "apples").is_err());

    // But the text `NaN` is a value the reference reads, and it carries all
    // the way through the arithmetic.
    assert_eq!(
        to_filetime(SECONDS, "Decimal", "NaN"),
        Ok("NaN".to_owned()),
        "the word NaN is read, where other unreadable text is refused"
    );
}

#[test]
fn a_mixed_case_filetime_is_refused() {
    // The hexadecimal reading matches the whole string against one alphabet.
    assert!(to_unix(SECONDS, "Hex (big endian)", "19db1DED53E8000").is_err());
    assert_eq!(
        to_unix(SECONDS, "Hex (big endian)", "19db1ded53e8000"),
        Ok("0".to_owned())
    );
    assert_eq!(
        to_unix(SECONDS, "Hex (big endian)", "19DB1DED53E8000"),
        Ok("0".to_owned())
    );
}

#[test]
fn an_unknown_unit_is_refused_and_the_micro_sign_is_the_greek_letter() {
    assert!(to_filetime("Fortnights", "Decimal", "1").is_err());

    // U+00B5, the micro sign, and U+03BC, the Greek letter, are drawn
    // identically. The reference uses the second, so an argument carrying the
    // first names no unit at all -- which is a failure a reader cannot see by
    // looking at the two strings.
    assert!(to_filetime("Microseconds (\u{00b5}s)", "Decimal", "1").is_err());
    assert_eq!(
        to_filetime(MICROSECONDS, "Decimal", "1"),
        Ok("116444736000000010".to_owned())
    );
}

#[test]
fn an_empty_input_answers_an_empty_string_rather_than_refusing() {
    for format in ["Decimal", "Hex (big endian)", "Hex (little endian)"] {
        assert_eq!(to_filetime(SECONDS, format, ""), Ok(String::new()));
        assert_eq!(to_unix(SECONDS, format, ""), Ok(String::new()));
    }
}

#[test]
fn an_unrecognised_format_is_read_as_decimal() {
    // The reference tests `format.startsWith("Hex")` rather than listing the
    // three names its interface offers, so a fourth name behaves as decimal.
    // Reproduced rather than tightened: refusing here would be a divergence
    // dressed up as strictness.
    assert_eq!(
        to_filetime(SECONDS, "Octal", "0"),
        to_filetime(SECONDS, "Decimal", "0")
    );
}

#[test]
fn the_two_endianness_swaps_are_different_rules_that_still_agree_on_the_value() {
    // Written out because the pair looks broken and is not. The epoch itself
    // renders as fifteen hexadecimal characters -- an odd count, which is
    // exactly where the two rules stop matching.
    let big = to_filetime(SECONDS, "Hex (big endian)", "0").expect("a filetime");
    assert_eq!(big, "19db1ded53e8000", "fifteen characters, an odd count");

    // Going out, the pairs reverse and a `0` and the *leading* digit are
    // appended at the end. Not what padding to an even length and reversing
    // would give, which is the tidy implementation to avoid.
    let little = to_filetime(SECONDS, "Hex (little endian)", "0").expect("a filetime");
    assert_eq!(little, "00803ed5deb19d01");

    // Coming back, the rule is different again: an odd-length input moves its
    // *trailing* character to the front. Here the input is even, so the pairs
    // simply reverse -- and what comes back is the original with a leading
    // zero in front of it. A different string, and the same number.
    assert_eq!(
        to_unix(SECONDS, "Hex (little endian)", &little),
        Ok("0".to_owned()),
        "the value survives the round trip even though the string does not"
    );

    // The asymmetry itself, on a short value where it is easy to read: five
    // characters out become six, and the six come back as those five behind a
    // zero rather than as the five themselves.
    assert_eq!(
        to_unix(SECONDS, "Hex (little endian)", "abcde"),
        to_unix(SECONDS, "Hex (big endian)", "ecdab"),
        "an odd-length input is not padded, it is rearranged"
    );
}
