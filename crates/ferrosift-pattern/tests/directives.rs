//! `#pragma` lines, and the two things that name another source.
//!
//! Nearly every pattern anyone has published opens with several pragmas — an
//! author, a description, a MIME type, a magic signature. Treating `#` as an
//! unsupported character refused 302 of the 308 patterns in
//! `WerWolv/ImHex-Patterns` on their first line, before the language had a
//! chance to be the reason.

use ferrosift_pattern::{Endian, EvalOptions, NodeValue, evaluate, parse};

#[test]
fn pragmas_are_kept_and_handed_on() {
    let pattern = parse(
        "#pragma author WerWolv\n\
         #pragma description 7z Archive\n\
         #pragma MIME application/x-7z-compressed\n\
         u8 first @ 0x00;",
    )
    .expect("a pattern that opens with pragmas parses");

    assert_eq!(pattern.directives.len(), 3);
    assert_eq!(pattern.directives[0].name, "author");
    assert_eq!(pattern.directives[0].argument, "WerWolv");
    assert_eq!(pattern.directives[1].name, "description");
    // The rest of the line, whole: a description is prose and running a lexer
    // over it would find an apostrophe and call it an unterminated literal.
    assert_eq!(pattern.directives[1].argument, "7z Archive");
    assert_eq!(pattern.directives[2].name, "MIME");
    assert_eq!(
        pattern.directives[2].argument,
        "application/x-7z-compressed"
    );
    assert_eq!(pattern.directives[0].position.line, 1);
    assert_eq!(pattern.directives[2].position.line, 3);

    // The declarations are still there, and unaffected.
    assert_eq!(pattern.declarations.len(), 1);
}

#[test]
fn a_pragma_may_carry_anything_a_line_can_hold() {
    // Two apostrophes that are not quotes, a colon that is not a symbol, and a
    // hash that does not open a second directive.
    let pattern = parse(
        "#pragma description Assassin's Creed: Unity's .forge archive #2\n\
         u8 a @ 0;",
    )
    .expect("prose is prose");
    assert_eq!(
        pattern.directives[0].argument,
        "Assassin's Creed: Unity's .forge archive #2"
    );
}

#[test]
fn a_pragma_with_no_argument_does_not_swallow_the_next_line() {
    let pattern =
        parse("#pragma once\n#pragma author Someone\nu8 a @ 0;").expect("an include guard parses");
    assert_eq!(pattern.directives.len(), 2);
    assert_eq!(pattern.directives[0].name, "once");
    assert_eq!(pattern.directives[0].argument, "");
    assert_eq!(pattern.directives[1].name, "author");
}

#[test]
fn pragma_endian_decides_the_default_byte_order() {
    // The one pragma in this subset that changes what a read produces, so the
    // one that is acted on rather than only recorded.
    let source = "#pragma endian big\nu16 value @ 0x00;";
    let pattern = parse(source).expect("parses");
    assert_eq!(pattern.endian, Some(Endian::Big));

    let nodes = evaluate(&pattern, &[0xca, 0xfe], &EvalOptions::default()).expect("evaluates");
    assert_eq!(nodes[0].value, NodeValue::Unsigned(0xcafe));
}

#[test]
fn the_pattern_wins_over_the_caller_s_default() {
    // `EvalOptions::endian` is a default for patterns that say nothing. A
    // pattern that says `big` is making a statement about the format it
    // describes, and honouring the caller instead would read every field
    // backwards.
    let pattern = parse("#pragma endian big\nu16 value @ 0x00;").expect("parses");
    let little = EvalOptions {
        endian: Endian::Little,
        ..EvalOptions::default()
    };
    let nodes = evaluate(&pattern, &[0xca, 0xfe], &little).expect("evaluates");
    assert_eq!(nodes[0].value, NodeValue::Unsigned(0xcafe));

    // And a field's own prefix still wins over the pragma, which is the same
    // order every other endianness decision follows.
    let mixed = parse("#pragma endian big\nle u16 value @ 0x00;").expect("parses");
    let nodes = evaluate(&mixed, &[0xca, 0xfe], &EvalOptions::default()).expect("evaluates");
    assert_eq!(nodes[0].value, NodeValue::Unsigned(0xfeca));
}

#[test]
fn an_unrecognised_pragma_is_recorded_rather_than_refused() {
    // The set of pragma names is upstream's and open. Refusing one this crate
    // does not know would refuse the whole pattern over a line that only
    // describes it.
    let pattern = parse("#pragma something_new whatever\nu8 a @ 0;").expect("parses");
    assert_eq!(pattern.directives[0].name, "something_new");
    assert_eq!(pattern.endian, None);

    // The same for an endian value that is neither `big` nor `little`.
    let odd = parse("#pragma endian sideways\nu8 a @ 0;").expect("parses");
    assert_eq!(odd.endian, None);
}

#[test]
fn naming_another_source_is_refused_under_its_own_code() {
    // Both spellings of the same limit: this crate reads one source and has no
    // filesystem to fetch another from. It is a fact about where the crate
    // runs rather than about the language, and 268 of the 308 published
    // patterns hit it — which is worth reporting as itself rather than as
    // "expected `;`, found `.`".
    for source in [
        "#include <std/mem.pat>\nu8 a @ 0;",
        "import std.io;\nu8 a @ 0;",
        "import type.magic;\nstruct S { u8 a; };",
    ] {
        let error = parse(source).expect_err("naming another source is refused");
        assert_eq!(
            error.code(),
            "pattern.parse.unsupported_directive",
            "{source}"
        );
    }
}

#[test]
fn a_pattern_with_no_directives_says_so() {
    let pattern = parse("u8 a @ 0;").expect("parses");
    assert!(pattern.directives.is_empty());
    assert_eq!(pattern.endian, None);
}
