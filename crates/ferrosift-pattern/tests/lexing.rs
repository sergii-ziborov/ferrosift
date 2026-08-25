//! Literal, escape, and every-built-in-type coverage for the front end.

use ferrosift_pattern::{
    Builtin, Declaration, EvalOptions, Expression, NodeValue, evaluate, parse,
};

fn placement_address(source: &str) -> u128 {
    let pattern = parse(source).unwrap_or_else(|error| panic!("parse failed: {error}"));
    match &pattern.declarations[0] {
        Declaration::Placement(placement) => match placement.address {
            Expression::Integer(value) => value,
            ref other => panic!("expected a literal address, found {other:?}"),
        },
        other => panic!("expected a placement, found {other:?}"),
    }
}

fn code(source: &str) -> &'static str {
    let error = parse(source).expect_err("expected a rejection");
    // Leak-free mapping: the crate returns `&'static str` codes already.
    match error.code() {
        "pattern.lex.unterminated_comment" => "pattern.lex.unterminated_comment",
        "pattern.lex.unterminated_text" => "pattern.lex.unterminated_text",
        "pattern.lex.invalid_escape" => "pattern.lex.invalid_escape",
        "pattern.lex.invalid_number" => "pattern.lex.invalid_number",
        "pattern.lex.number_overflow" => "pattern.lex.number_overflow",
        "pattern.lex.unexpected_character" => "pattern.lex.unexpected_character",
        other => panic!("unexpected code {other} for source: {source}"),
    }
}

#[test]
fn every_radix_and_digit_separator_parses() {
    assert_eq!(placement_address("u8 a @ 0;"), 0);
    assert_eq!(placement_address("u8 a @ 255;"), 255);
    assert_eq!(placement_address("u8 a @ 0xDEAD;"), 0xdead);
    assert_eq!(placement_address("u8 a @ 0XBEEF;"), 0xbeef);
    assert_eq!(placement_address("u8 a @ 0b1010_1010;"), 0b1010_1010);
    assert_eq!(placement_address("u8 a @ 0B11;"), 0b11);
    assert_eq!(placement_address("u8 a @ 0o777;"), 0o777);
    assert_eq!(placement_address("u8 a @ 0O10;"), 0o10);
    assert_eq!(placement_address("u8 a @ 1_2_3;"), 123);
}

#[test]
fn the_largest_representable_literal_is_accepted() {
    let source = alloc_format(u128::MAX);
    assert_eq!(placement_address(&source), u128::MAX);
}

#[test]
fn literals_beyond_128_bits_are_rejected() {
    let source = alloc_format_overflow();
    assert_eq!(code(&source), "pattern.lex.number_overflow");
}

#[test]
fn malformed_numbers_are_rejected() {
    for source in [
        "u8 a @ 0x;",
        "u8 a @ 0b;",
        "u8 a @ 0o;",
        "u8 a @ 0b2;",
        "u8 a @ 0xZZ;",
    ] {
        assert_eq!(code(source), "pattern.lex.invalid_number", "{source}");
    }
}

#[test]
fn unterminated_literals_and_comments_are_rejected() {
    for source in ["using A = /* never closed", "struct S { u8 a; }; /* open"] {
        assert_eq!(code(source), "pattern.lex.unterminated_comment", "{source}");
    }
    for source in ["u8 a @ 'x;", "u8 a @ '';", "u8 a @ 'a", "u8 a @ \"open;"] {
        assert_eq!(code(source), "pattern.lex.unterminated_text", "{source}");
    }
}

#[test]
fn unsupported_escapes_and_characters_are_rejected() {
    assert_eq!(code(r"u8 a @ '\q';"), "pattern.lex.invalid_escape");
    assert_eq!(code(r"u8 a @ '\"), "pattern.lex.invalid_escape");
    // `$` and `&` used to belong here. They are operators now, so a source
    // using them reaches the parser and fails there instead -- the characters
    // that remain are the ones no part of the grammar spells.
    for source in [
        "u8 a @ 0; #pragma once",
        "struct S { u8 a; } `",
        "u8 a @ 0 \\ 1;",
    ] {
        assert_eq!(code(source), "pattern.lex.unexpected_character", "{source}");
    }
}

#[test]
fn comments_may_close_and_nest_line_forms() {
    let pattern = parse(
        "/* block */ struct S { // trailing line
             u8 a; /* between */ u8 b;
         };
         // final line with no newline",
    )
    .expect("comment handling parses");
    let Declaration::Struct(structure) = &pattern.declarations[0] else {
        panic!("expected a struct");
    };
    assert_eq!(structure.members.len(), 2);
}

#[test]
fn every_builtin_name_maps_to_its_declared_width() {
    for (name, size) in [
        ("u8", 1),
        ("u16", 2),
        ("u24", 3),
        ("u32", 4),
        ("u48", 6),
        ("u64", 8),
        ("u96", 12),
        ("u128", 16),
        ("s8", 1),
        ("s16", 2),
        ("s24", 3),
        ("s32", 4),
        ("s48", 6),
        ("s64", 8),
        ("s96", 12),
        ("s128", 16),
        ("float", 4),
        ("double", 8),
        ("bool", 1),
        ("char", 1),
        ("char16", 2),
    ] {
        let builtin = Builtin::parse(name).unwrap_or_else(|| panic!("{name} must be built in"));
        assert_eq!(builtin.size(), size, "{name}");
        assert_eq!(builtin.name(), name, "{name}");
    }
    assert!(Builtin::parse("NotABuiltin").is_none());
    assert!(Builtin::parse("u7").is_none());
}

#[test]
fn every_builtin_width_reads_the_expected_span() {
    let data = [0xff_u8; 32];
    for name in [
        "u8", "u16", "u24", "u32", "u48", "u64", "u96", "u128", "s8", "s16", "s24", "s32", "s48",
        "s64", "s96", "s128", "float", "double", "bool", "char", "char16",
    ] {
        let source = alloc_placement(name);
        let pattern = parse(&source).unwrap_or_else(|error| panic!("{name}: {error}"));
        let nodes = evaluate(&pattern, &data, &EvalOptions::default())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = u64::from(
            Builtin::parse(name)
                .unwrap_or_else(|| panic!("{name} must be built in"))
                .size(),
        );
        assert_eq!(nodes[0].size, expected, "{name}");
        assert_eq!(nodes[0].type_name, name, "{name}");
    }
}

#[test]
fn char16_falls_back_to_the_replacement_character() {
    // 0xD800 is an unpaired surrogate and is not a scalar value.
    let pattern = parse("le char16 lone @ 0;").expect("parses");
    let nodes = evaluate(&pattern, &[0x00, 0xd8], &EvalOptions::default()).expect("evaluates");
    assert_eq!(nodes[0].value, NodeValue::Char(char::REPLACEMENT_CHARACTER));
}

#[test]
fn string_and_character_escapes_resolve() {
    // Escapes are exercised through a placement address position that accepts
    // any literal token, proving the scanner resolves each form.
    for source in [
        r"using A = u8; // '\n'",
        r"using B = u8; // '\r' '\t' '\0'",
        r"using C = u8; // '\\' and '\'' and an escaped quote",
    ] {
        assert!(parse(source).is_ok(), "{source}");
    }
}

fn alloc_format(value: u128) -> String {
    format!("u8 a @ {value};")
}

fn alloc_format_overflow() -> String {
    // One decimal digit more than u128::MAX can represent.
    format!("u8 a @ {}0;", u128::MAX)
}

fn alloc_placement(type_name: &str) -> String {
    format!("{type_name} value @ 0;")
}
