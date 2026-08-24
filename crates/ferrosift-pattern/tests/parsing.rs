//! Grammar acceptance and rejection vectors for the supported subset.

use ferrosift_pattern::{Builtin, Declaration, Endian, Pattern, PatternError, TypeKind, parse};

fn accept(source: &str) -> Pattern {
    parse(source).unwrap_or_else(|error| panic!("expected a parse: {error}"))
}

fn reject(source: &str) -> PatternError {
    parse(source).expect_err("expected a rejection")
}

fn as_struct(pattern: &Pattern, name: &str) -> ferrosift_pattern::StructDeclaration {
    match pattern.type_named(name).expect("declared type") {
        Declaration::Struct(value) => value.clone(),
        other => panic!("expected a struct, found {other:?}"),
    }
}

#[test]
fn parses_a_struct_of_builtin_fields() {
    let pattern = accept("struct Header { u32 magic; u16 version; char tag; };");
    let header = as_struct(&pattern, "Header");

    assert_eq!(header.fields.len(), 3);
    assert_eq!(header.fields[0].name, "magic");
    assert_eq!(
        header.fields[0].type_reference.kind,
        TypeKind::Builtin(Builtin::Unsigned(4))
    );
    assert_eq!(
        header.fields[2].type_reference.kind,
        TypeKind::Builtin(Builtin::Char)
    );
    assert!(
        header
            .fields
            .iter()
            .all(|field| field.array_length.is_none())
    );
}

#[test]
fn parses_fixed_size_arrays_and_named_field_types() {
    let pattern = accept(
        "struct Entry { u8 id; };
         struct Table { Entry rows[4]; u8 checksum; };",
    );
    let table = as_struct(&pattern, "Table");

    assert_eq!(table.fields[0].array_length, Some(4));
    assert_eq!(
        table.fields[0].type_reference.kind,
        TypeKind::Named("Entry".into())
    );
    assert_eq!(table.fields[1].array_length, None);
}

#[test]
fn parses_endianness_prefixes_on_types() {
    let pattern = accept("struct Mixed { be u32 network; le u32 host; u16 unspecified; };");
    let mixed = as_struct(&pattern, "Mixed");

    assert_eq!(mixed.fields[0].type_reference.endian, Some(Endian::Big));
    assert_eq!(mixed.fields[1].type_reference.endian, Some(Endian::Little));
    assert_eq!(mixed.fields[2].type_reference.endian, None);
}

#[test]
fn enum_values_continue_from_the_previous_entry() {
    let pattern = accept("enum Kind : u8 { Zero, One, Five = 5, Six, };");
    let Declaration::Enum(kind) = pattern.type_named("Kind").expect("declared type") else {
        panic!("expected an enum");
    };

    let values: Vec<_> = kind
        .entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.value))
        .collect();
    assert_eq!(values, [("Zero", 0), ("One", 1), ("Five", 5), ("Six", 6)]);
    assert_eq!(kind.backing.kind, TypeKind::Builtin(Builtin::Unsigned(1)));
}

#[test]
fn parses_bitfields_and_aliases() {
    let pattern = accept(
        "bitfield Flags { high : 3; middle : 4; low : 1; };
         using Word = be u16;",
    );

    let Declaration::Bitfield(flags) = pattern.type_named("Flags").expect("declared type") else {
        panic!("expected a bitfield");
    };
    let widths: Vec<_> = flags.members.iter().map(|member| member.bits).collect();
    assert_eq!(widths, [3, 4, 1]);

    let Declaration::Alias(word) = pattern.type_named("Word").expect("declared type") else {
        panic!("expected an alias");
    };
    assert_eq!(word.target.endian, Some(Endian::Big));
    assert_eq!(word.target.kind, TypeKind::Builtin(Builtin::Unsigned(2)));
}

#[test]
fn parses_placements_with_every_supported_radix() {
    let pattern = accept(
        "struct S { u8 a; };
         S at_hex @ 0x10;
         S at_binary @ 0b1000;
         S at_octal @ 0o17;
         S at_decimal @ 42;
         S grouped @ 1_000;",
    );

    let addresses: Vec<_> = pattern
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Placement(value) => Some(value.address),
            _ => None,
        })
        .collect();
    assert_eq!(addresses, [0x10, 0b1000, 0o17, 42, 1000]);
}

#[test]
fn comments_and_whitespace_are_ignored() {
    let pattern = accept(
        "// leading line comment
         struct S { /* inline */ u8 a; } ;
         /* trailing
            block comment */",
    );
    assert_eq!(as_struct(&pattern, "S").fields.len(), 1);
}

#[test]
fn arrays_of_placements_are_recorded() {
    let pattern = accept("u32 values[8] @ 0x40;");
    let Declaration::Placement(placement) = &pattern.declarations[0] else {
        panic!("expected a placement");
    };
    assert_eq!(placement.array_length, Some(8));
    assert_eq!(placement.address, 0x40);
}

#[test]
fn malformed_sources_report_stable_codes() {
    for (source, code) in [
        ("struct S { u8 a; ", "pattern.parse.unexpected_token"),
        ("struct S { u8 a }", "pattern.parse.expected_symbol"),
        ("struct { u8 a; };", "pattern.parse.expected_identifier"),
        ("u8 a @ ;", "pattern.parse.expected_integer"),
        ("struct S { 4 a; };", "pattern.parse.expected_type"),
        ("u8 a[0] @ 0;", "pattern.parse.invalid_array_length"),
        ("bitfield B { a : 0; };", "pattern.parse.invalid_bit_width"),
        ("bitfield B { a : 65; };", "pattern.parse.invalid_bit_width"),
        (
            "struct S { u8 a; }; struct S { u8 b; };",
            "pattern.parse.duplicate_declaration",
        ),
        (
            "struct S { u8 a; }; /* open",
            "pattern.lex.unterminated_comment",
        ),
        // A radix prefix with no digits fails in the lexer, before the parser
        // ever asks for an integer.
        ("u8 a @ 0x; ", "pattern.lex.invalid_number"),
        ("struct S { u8 a; } $", "pattern.lex.unexpected_character"),
    ] {
        let error = reject(source);
        assert_eq!(error.code(), code, "source: {source}");
    }
}

#[test]
fn errors_carry_a_one_based_source_position() {
    let error = reject("struct S {\n    u8 a\n};");
    assert_eq!(error.code(), "pattern.parse.expected_symbol");
    assert_eq!(error.position().line, 3);
    assert_eq!(error.position().column, 1);
}
