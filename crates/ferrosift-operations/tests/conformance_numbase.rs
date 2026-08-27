//! What the base conversions refuse, and where the two stop being inverses.
//!
//! The corpus pins outputs, so it cannot pin an input the reference throws on
//! -- and reading a number in another base throws for three separate reasons,
//! each of which a port could plausibly have answered instead.
//!
//! It also holds the asymmetry between the pair. `To Base` hands its value to
//! the reference's `toString(base)`. `From Base` does *not* hand its text to
//! the matching constructor: it splits on the point and reads each fractional
//! digit alone. So a value whose letters mix case is refused before the point
//! and read after it, in one operation, on one input.

#![cfg(feature = "bignum")]

use ferrosift_model::{ArgumentValue, Value};

mod support;

fn read(text: &str, radix: i128) -> Result<String, ()> {
    let arguments = support::argument("radix", ArgumentValue::Integer(radix));
    match support::run_with_budget(
        "encoding.radix.decode@1",
        arguments,
        support::text(text),
        support::budget(),
    ) {
        Ok(result) => match result.value {
            Value::Decimal(value) => Ok(value.to_fixed()),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

fn write(text: &str, radix: i128) -> Result<String, ()> {
    let arguments = support::argument("radix", ArgumentValue::Integer(radix));
    match support::run_with_budget(
        "encoding.radix.encode@1",
        arguments,
        support::text(text),
        support::budget(),
    ) {
        Ok(result) => match result.value {
            Value::Text(value) => Ok(value.text),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

#[test]
fn a_digit_outside_the_base_is_refused() {
    for (text, radix) in [("102", 2), ("8", 8), ("ff", 10), ("1p5", 16), ("1e5", 14)] {
        assert!(
            read(text, radix).is_err(),
            "{text:?} has no reading in base {radix}"
        );
    }
}

#[test]
fn a_prefix_the_ordinary_reading_requires_is_refused_with_a_base() {
    // `0x` is how the single-argument constructor is *told* the base. Giving
    // the base explicitly makes the prefix two stray digits instead.
    assert!(read("0xff", 16).is_err());
    assert!(read("0b101", 2).is_err());
    assert_eq!(read("ff", 16), Ok("255".to_owned()));
}

#[test]
fn mixed_case_is_refused_before_the_point_and_read_after_it() {
    // The whole of the integer part is matched against one alphabet, so its
    // letters must agree. Each fractional digit is read on its own, and a
    // single digit has no case to disagree with -- which is why `1F.aB` is a
    // number and `Ff` is not.
    assert!(read("Ff", 16).is_err(), "mixed case before the point");
    assert!(read("aBc", 16).is_err());
    assert_eq!(read("ff", 16), Ok("255".to_owned()));
    assert_eq!(read("FF", 16), Ok("255".to_owned()));

    assert_eq!(
        read("1F.aB", 16),
        Ok("31.66796875".to_owned()),
        "mixed case after the point is read one digit at a time"
    );
    assert_eq!(read("1f.Ab", 16), Ok("31.66796875".to_owned()));
}

#[test]
fn a_base_outside_the_range_is_refused_in_both_directions() {
    for radix in [-1, 0, 1, 37, 100] {
        assert!(read("1", radix).is_err(), "reading in base {radix}");
        assert!(write("1", radix).is_err(), "writing in base {radix}");
    }
}

#[test]
fn nothing_at_all_is_zero_with_a_base_and_not_a_number_without_one() {
    // The reading takes a base, so an empty string is zero. The *writing*
    // takes its value from the dish, which reads the empty input through the
    // single-argument constructor, catches the exception, and substitutes
    // not-a-number -- so the same input answers differently in each
    // direction, and both are the reference.
    assert_eq!(read("", 16), Ok("0".to_owned()));
    assert_eq!(read("   ", 16), Ok("0".to_owned()));
    assert_eq!(write("", 16), Ok("NaN".to_owned()));
    assert_eq!(write("apples", 16), Ok("NaN".to_owned()));
}

#[test]
fn a_tie_rounds_down_on_an_odd_base_and_up_on_an_even_one() {
    // The reference decides the rounding from the twenty-first digit alone,
    // against half the base as a real number. No digit of an odd base is
    // worth exactly half, so a value sitting exactly on the boundary
    // truncates -- the opposite of every other rounding in the port.
    assert_eq!(
        write("0.1", 5),
        Ok("0.02222222222222222222".to_owned()),
        "a tenth in base five repeats and sits exactly half a place above the last digit"
    );
    assert_eq!(
        write("0.1", 2),
        Ok("0.0001100110011001101".to_owned()),
        "and in base two the deciding digit is a whole half, which rounds away from zero"
    );
}
