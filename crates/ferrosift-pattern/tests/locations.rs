//! Where an evaluation failure happened, in both of the places it can be.
//!
//! A failure has two locations and neither answers the other's question. The
//! *source position* is a line and column in the pattern; the *data offset* is
//! a byte in the subject. "The read left the data" is useless without knowing
//! which byte was wanted, and knowing the byte is useless without knowing
//! which line asked for it.
//!
//! Every evaluation error used to report `0:0` — a placeholder that said
//! nothing at all, and looked authoritative while saying it.

use ferrosift_pattern::{EvalOptions, PatternError, evaluate, parse};

fn failure(source: &str, data: &[u8]) -> PatternError {
    let pattern = parse(source).unwrap_or_else(|error| panic!("parse failed: {error}"));
    evaluate(&pattern, data, &EvalOptions::default())
        .expect_err("this pattern is meant to fail against this data")
}

#[test]
fn a_read_past_the_end_names_the_byte_it_wanted() {
    // Three bytes of data and a four-byte field at offset two: the read wants
    // byte 2 and there are only bytes 0 through 2.
    let error = failure("be u32 value @ 0x02;", &[1, 2, 3]);

    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
    assert_eq!(error.data_offset(), Some(2));
    assert_eq!(error.position().line, 1);
    assert!(
        error.to_string().contains("data offset 0x2"),
        "the rendering should carry both locations: {error}"
    );
}

#[test]
fn the_reported_line_is_the_declaration_that_failed() {
    // The failing member is on line three. Reporting line five — the
    // placement that reached it — would send a reader to the one line in the
    // pattern that is definitely correct.
    let source = "struct Header {\n\
                  \x20   u8 kind;\n\
                  \x20   be u64 length;\n\
                  };\n\
                  Header header @ 0x00;";
    let error = failure(source, &[1, 2, 3]);

    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
    assert_eq!(error.position().line, 3, "{error}");
    // One byte of `kind` was read, so the failing field starts at offset one.
    assert_eq!(error.data_offset(), Some(1));
}

#[test]
fn a_nested_member_reports_its_own_line_rather_than_the_outer_one() {
    let source = "struct Inner {\n\
                  \x20   u8 a;\n\
                  \x20   be u32 b;\n\
                  };\n\
                  struct Outer {\n\
                  \x20   Inner inner;\n\
                  };\n\
                  Outer outer @ 0x00;";
    let error = failure(source, &[9, 9]);

    // Line 3 is `be u32 b;`. Lines 6 and 8 are the member and the placement
    // that reached it, and both are correct.
    assert_eq!(error.position().line, 3, "{error}");
    assert_eq!(error.data_offset(), Some(1));
}

#[test]
fn a_placement_address_past_the_data_reports_the_placement() {
    let source = "u8 first @ 0x00;\nbe u16 second @ 0x40;";
    let error = failure(source, &[1, 2]);

    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
    assert_eq!(error.position().line, 2, "{error}");
    assert_eq!(error.data_offset(), Some(0x40));
}

#[test]
fn an_array_element_names_the_element_that_left_the_data() {
    // Four two-byte elements from offset zero need eight bytes; five are
    // there, so the read that fails is the block starting at zero.
    let error = failure("be u16 words[4] @ 0x00;", &[1, 2, 3, 4, 5]);

    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
    assert_eq!(error.data_offset(), Some(0));
    assert_eq!(error.position().line, 1);
}

#[test]
fn a_failure_about_the_pattern_rather_than_the_bytes_carries_no_offset() {
    // An undeclared type is wrong wherever the data happens to be, so there is
    // no byte to point at — and inventing one would be worse than saying
    // nothing. The line is still reported.
    let error = failure("Missing thing @ 0x00;", &[1, 2, 3, 4]);

    assert_eq!(error.code(), "pattern.eval.unknown_type");
    assert_eq!(error.data_offset(), None);
    assert_eq!(error.position().line, 1);
    assert!(
        !error.to_string().contains("data offset"),
        "an error with no byte must not name one: {error}"
    );
}

#[test]
fn a_parse_failure_keeps_the_position_it_always_had() {
    // Nothing here changed for the parser, which has reported real positions
    // all along. The assertion is that adding a second location did not
    // disturb the first.
    let error =
        parse("struct S { u8 a; };\nS s @ ;").expect_err("a missing address is a parse error");

    assert!(error.code().starts_with("pattern.parse."), "{error}");
    assert_eq!(error.position().line, 2);
    assert_eq!(error.data_offset(), None);
}

#[test]
fn an_unknown_position_renders_as_unknown_rather_than_as_line_zero() {
    // Line and column are one-based, so zero is the value no real position can
    // take. A reader who saw `0:0` would go looking for a line zero.
    let position = ferrosift_pattern::Position::UNKNOWN;
    assert!(!position.is_known());
    assert_eq!(position.to_string(), "?:?");

    let known = ferrosift_pattern::Position { line: 4, column: 9 };
    assert!(known.is_known());
    assert_eq!(known.to_string(), "4:9");
}
