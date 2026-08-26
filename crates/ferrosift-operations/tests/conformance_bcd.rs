//! What binary-coded decimal refuses, and the rules that read like mistakes.
//!
//! The corpus pins outputs, so it cannot pin the four separate reasons this
//! pair throws. Nor can it state, in one place, the three rules a tidier
//! implementation would quietly get wrong — each is visible in the corpus only
//! as a byte string that happens to be right.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn settings(scheme: &str, packed: bool, signed: bool, format: &str) -> Arguments {
    Arguments::from([
        ("scheme".into(), ArgumentValue::Text(scheme.into())),
        ("packed".into(), ArgumentValue::Boolean(packed)),
        ("signed".into(), ArgumentValue::Boolean(signed)),
        ("format".into(), ArgumentValue::Text(format.into())),
    ])
}

fn encode(input: &str, packed: bool, signed: bool, format: &str) -> Result<String, ()> {
    let arguments = settings("8 4 2 1", packed, signed, format);
    match support::run_with_budget(
        "encoding.bcd.encode@1",
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

fn decode(input: &str, packed: bool, signed: bool, format: &str) -> Result<String, ()> {
    let arguments = settings("8 4 2 1", packed, signed, format);
    match support::run_with_budget(
        "encoding.bcd.decode@1",
        arguments,
        support::text(input),
        support::budget(),
    ) {
        Ok(result) => match result.value {
            Value::Decimal(value) => Ok(value.to_fixed()),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

#[test]
fn a_fraction_is_refused_and_a_whole_number_is_not() {
    // There is no nibble for a decimal point, and the reference says so
    // rather than truncating -- which would be the plausible wrong answer.
    for input in ["1.5", "0.1", "-2.25"] {
        assert!(encode(input, true, false, "Nibbles").is_err(), "{input}");
    }
    assert_eq!(
        encode("15", true, false, "Nibbles"),
        Ok("0001 0101".to_owned())
    );
}

#[test]
fn text_that_is_not_a_number_is_refused() {
    // The dish turns unreadable text into not-a-number before the operation
    // sees it, and the operation refuses that outright.
    for input in ["NaN", "apples", ""] {
        assert!(encode(input, true, false, "Raw").is_err(), "{input:?}");
    }
}

#[test]
fn an_infinity_is_encoded_as_the_letters_of_its_own_name() {
    // Not a refusal, which is the surprise. The reference's guard is that
    // rounding the value down leaves it unchanged, and that is true of an
    // infinity -- so it renders as `Infinity`, and each of those eight
    // letters indexes the digit table with not-a-number.
    //
    // Packed, the missing digits contribute nothing and the answer is four
    // zero bytes. The two binary renderings call a method on the missing
    // digit and throw, which is why only some combinations produce anything.
    assert_eq!(
        encode("Infinity", true, false, "Raw"),
        Ok("\u{0}\u{0}\u{0}\u{0}".to_owned()),
        "eight letters, none of them a digit, packed two to a byte"
    );
    assert_eq!(
        encode("Infinity", true, false, "Bytes"),
        Ok("00000000 00000000 00000000 00000000".to_owned())
    );
    assert!(
        encode("Infinity", true, false, "Nibbles").is_err(),
        "the nibble rendering has nothing to write for a missing digit"
    );
    assert!(encode("Infinity", false, false, "Bytes").is_err());

    // And the sign nibble still lands, from a numeric comparison against
    // zero: an infinity is above it and a negative infinity is not.
    assert_eq!(
        encode("Infinity", true, true, "Raw"),
        Ok("\u{0}\u{0}\u{0}\u{0}\u{c}".to_owned())
    );
    assert_eq!(
        encode("-Infinity", true, true, "Raw"),
        Ok("\u{0}\u{0}\u{0}\u{0}\u{d}".to_owned())
    );
}

#[test]
fn zero_takes_the_negative_sign_nibble() {
    // The sign comes from `value > 0`, not from the sign of the value, so
    // zero is written as a debit. A port that asked whether the value was
    // negative would answer credit and be wrong on exactly one input.
    assert_eq!(
        encode("0", true, true, "Nibbles"),
        Ok("0000 1101".to_owned()),
        "one digit, so no leading zero -- then debit, for a value not above zero"
    );
    assert_eq!(
        encode("1", true, true, "Nibbles"),
        Ok("0001 1100".to_owned()),
        "the same shape, and credit, for a value that is"
    );
}

#[test]
fn a_sign_nibble_forces_a_leading_zero_only_when_packed_and_even() {
    // Otherwise the sign would sit alone in the last byte and a reader could
    // not tell whether the value ended in a zero. Unpacked, every nibble has
    // its own byte and the question never arises.
    assert_eq!(
        encode("1234", true, true, "Nibbles"),
        Ok("0000 0001 0010 0011 0100 1100".to_owned()),
        "even and packed: a zero is prepended"
    );
    assert_eq!(
        encode("12345", true, true, "Nibbles"),
        Ok("0001 0010 0011 0100 0101 1100".to_owned()),
        "odd and packed: nothing is prepended"
    );
    assert_eq!(
        encode("1234", false, true, "Nibbles"),
        Ok("0000 0001 0000 0010 0000 0011 0000 0100 0000 1100".to_owned()),
        "even and unpacked: nothing is prepended either"
    );
}

#[test]
fn an_unpacked_reading_keeps_the_second_of_every_pair_and_drops_an_odd_tail() {
    // The reference removes an element and then advances past the next one,
    // which is usually a bug and is the behaviour here. The consequence worth
    // stating is the tail: three nibbles come back as one, not two.
    assert_eq!(
        decode("0001 0010 0011", false, false, "Nibbles"),
        Ok("2".to_owned()),
        "three nibbles, and only the second survives"
    );
    assert_eq!(
        decode("0000 0001 0010 0011", false, false, "Nibbles"),
        Ok("13".to_owned()),
        "four nibbles, and the second of each pair survives"
    );
}

#[test]
fn a_signed_reading_consumes_the_last_nibble_whatever_it_is() {
    // Only two values mean anything there, so a digit in that place is
    // dropped rather than read -- which is why the same input reads as three
    // digits unsigned and two signed.
    assert_eq!(
        decode("0001 0010 0011", true, false, "Nibbles"),
        Ok("123".to_owned())
    );
    assert_eq!(
        decode("0001 0010 0011", true, true, "Nibbles"),
        Ok("12".to_owned())
    );
    assert_eq!(
        decode("0001 0010 0011 1101", true, true, "Nibbles"),
        Ok("-123".to_owned())
    );
    assert_eq!(
        decode("0001 0010 0011 1100", true, true, "Nibbles"),
        Ok("123".to_owned())
    );
}

#[test]
fn a_short_final_group_is_read_as_what_it_holds() {
    // Eleven characters are three nibbles, and the last is worth one rather
    // than eight: the reference reads what is there instead of padding.
    assert_eq!(
        decode("0001 0010 001", true, false, "Nibbles"),
        Ok("121".to_owned())
    );
    assert_eq!(
        decode("0001001", true, false, "Nibbles"),
        Ok("11".to_owned())
    );
}

#[test]
fn a_nibble_no_scheme_has_a_digit_for_is_refused() {
    assert!(decode("1111 1110", true, false, "Nibbles").is_err());
    // And the schemes disagree about which nibbles those are, so the same
    // input reads in one and is refused by another.
    let arguments = settings("8 4 -2 -1", true, false, "Nibbles");
    assert!(
        support::run_with_budget(
            "encoding.bcd.decode@1",
            arguments,
            support::text("0001 0010 0011"),
            support::budget(),
        )
        .is_err(),
        "one is not a digit in the negative-weight scheme"
    );
}

#[test]
fn nothing_to_read_is_refused_rather_than_answered_as_zero() {
    for (input, packed, signed) in [("", true, false), ("   ", true, false)] {
        assert!(
            decode(input, packed, signed, "Nibbles").is_err(),
            "{input:?}"
        );
    }
    // And a reading that consumes its only nibble as a sign leaves no digits
    // at all, which the constructor refuses rather than calling zero.
    assert!(decode("1100", true, true, "Nibbles").is_err());
}

#[test]
fn an_unknown_scheme_is_refused_in_both_directions() {
    let arguments = settings("9 9 9 9", true, false, "Nibbles");
    assert!(
        support::run_with_budget(
            "encoding.bcd.encode@1",
            arguments.clone(),
            support::text("1"),
            support::budget(),
        )
        .is_err()
    );
    assert!(
        support::run_with_budget(
            "encoding.bcd.decode@1",
            arguments,
            support::text("0001"),
            support::budget(),
        )
        .is_err()
    );
}
