//! Grammar acceptance and rejection vectors for the supported subset.

use ferrosift_pattern::{
    ArrayLength, BinaryOperator, Builtin, Declaration, Endian, Expression, Field, Member, Pattern,
    PatternError, SizeOfTarget, TypeKind, parse,
};

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

/// The plain fields of a member list, skipping conditionals and padding.
fn fields(members: &[Member]) -> Vec<&Field> {
    members
        .iter()
        .filter_map(|member| match member {
            Member::Field(field) => Some(field),
            _ => None,
        })
        .collect()
}

/// A fixed array length written as a literal.
fn counted(value: u128) -> ArrayLength {
    ArrayLength::Counted(Expression::Integer(value))
}

#[test]
fn parses_a_struct_of_builtin_fields() {
    let pattern = accept("struct Header { u32 magic; u16 version; char tag; };");
    let header = as_struct(&pattern, "Header");
    let fields = fields(&header.members);

    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].name, "magic");
    assert_eq!(
        fields[0].type_reference.kind,
        TypeKind::Builtin(Builtin::Unsigned(4))
    );
    assert_eq!(
        fields[2].type_reference.kind,
        TypeKind::Builtin(Builtin::Char)
    );
    assert!(fields.iter().all(|field| field.array_length.is_none()));
}

#[test]
fn parses_fixed_size_arrays_and_named_field_types() {
    let pattern = accept(
        "struct Entry { u8 id; };
         struct Table { Entry rows[4]; u8 checksum; };",
    );
    let table = as_struct(&pattern, "Table");
    let fields = fields(&table.members);

    assert_eq!(fields[0].array_length, Some(counted(4)));
    assert_eq!(
        fields[0].type_reference.kind,
        TypeKind::Named("Entry".into())
    );
    assert_eq!(fields[1].array_length, None);
}

#[test]
fn parses_endianness_prefixes_on_types() {
    let pattern = accept("struct Mixed { be u32 network; le u32 host; u16 unspecified; };");
    let mixed = as_struct(&pattern, "Mixed");
    let fields = fields(&mixed.members);

    assert_eq!(fields[0].type_reference.endian, Some(Endian::Big));
    assert_eq!(fields[1].type_reference.endian, Some(Endian::Little));
    assert_eq!(fields[2].type_reference.endian, None);
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
fn enum_values_and_bit_widths_accept_constant_expressions() {
    let pattern = accept(
        "enum Flag : u16 { None = 0, Read = 1 << 0, Write = 1 << 1, Both = (1 << 0) | (1 << 1) };
         bitfield Packed { tag : 2 + 1; rest : 8 - 3; };",
    );

    let Declaration::Enum(flag) = pattern.type_named("Flag").expect("declared type") else {
        panic!("expected an enum");
    };
    let values: Vec<_> = flag.entries.iter().map(|entry| entry.value).collect();
    assert_eq!(values, [0, 1, 2, 3]);

    let Declaration::Bitfield(packed) = pattern.type_named("Packed").expect("declared type") else {
        panic!("expected a bitfield");
    };
    let widths: Vec<_> = packed.members.iter().map(|member| member.bits).collect();
    assert_eq!(widths, [3, 5]);
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
            Declaration::Placement(value) => Some(value.address.clone()),
            _ => None,
        })
        .collect();
    let expected: Vec<_> = [0x10, 0b1000, 0o17, 42, 1000]
        .into_iter()
        .map(Expression::Integer)
        .collect();
    assert_eq!(addresses, expected);
}

#[test]
fn comments_and_whitespace_are_ignored() {
    let pattern = accept(
        "// leading line comment
         struct S { /* inline */ u8 a; } ;
         /* trailing
            block comment */",
    );
    assert_eq!(as_struct(&pattern, "S").members.len(), 1);
}

#[test]
fn arrays_of_placements_are_recorded() {
    let pattern = accept("u32 values[8] @ 0x40;");
    let Declaration::Placement(placement) = &pattern.declarations[0] else {
        panic!("expected a placement");
    };
    assert_eq!(placement.array_length, Some(counted(8)));
    assert_eq!(placement.address, Expression::Integer(0x40));
}

#[test]
fn parses_unions() {
    let pattern = accept("union Value { u32 as_word; u8 as_bytes[4]; };");
    let Declaration::Union(value) = pattern.type_named("Value").expect("declared type") else {
        panic!("expected a union");
    };
    let fields = fields(&value.members);
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[1].array_length, Some(counted(4)));
}

#[test]
fn parses_conditional_members_including_else_if_chains() {
    let pattern = accept(
        "struct S {
             u8 kind;
             if (kind == 1) { u8 a; }
             else if (kind == 2) { u16 b; }
             else { u32 c; }
         };",
    );
    let s = as_struct(&pattern, "S");

    let Member::Conditional {
        condition,
        when_true,
        when_false,
    } = &s.members[1]
    else {
        panic!("expected a conditional");
    };
    assert!(matches!(
        condition,
        Expression::Binary {
            operator: BinaryOperator::Equal,
            ..
        }
    ));
    assert_eq!(fields(when_true)[0].name, "a");

    // `else if` nests as a conditional inside the false arm rather than
    // flattening, so the chain keeps the shape it was written in.
    let [Member::Conditional { when_false: tail, .. }] = &when_false[..] else {
        panic!("expected the else arm to hold one conditional");
    };
    assert_eq!(fields(tail)[0].name, "c");
}

#[test]
fn parses_padding_and_while_sized_arrays() {
    let pattern = accept(
        "struct S {
             u8 count;
             padding[3];
             u8 items[while($ < 16)];
         };",
    );
    let s = as_struct(&pattern, "S");

    assert_eq!(s.members[1], Member::Padding(Expression::Integer(3)));
    let Some(ArrayLength::While(condition)) = &fields(&s.members)[1].array_length else {
        panic!("expected a while-sized array");
    };
    assert!(matches!(
        condition,
        Expression::Binary {
            operator: BinaryOperator::Less,
            ..
        }
    ));
}

#[test]
fn parses_sizeof_over_builtins_and_paths() {
    let pattern = accept(
        "struct Header { u8 a; };
         Header header @ 0;
         u8 body[sizeof(u32)] @ sizeof(header);",
    );
    let Declaration::Placement(body) = &pattern.declarations[2] else {
        panic!("expected a placement");
    };
    assert_eq!(
        body.array_length,
        Some(ArrayLength::Counted(Expression::SizeOf(
            SizeOfTarget::Builtin(Builtin::Unsigned(4))
        )))
    );
    assert_eq!(
        body.address,
        Expression::SizeOf(SizeOfTarget::Path(vec!["header".into()]))
    );
}

#[test]
fn operator_precedence_follows_the_c_ladder() {
    let pattern = accept("u8 a[1 + 2 * 3] @ 0;");
    let Declaration::Placement(placement) = &pattern.declarations[0] else {
        panic!("expected a placement");
    };
    // Multiplication binds tighter, so the sum is the outer node. Asserting on
    // the tree rather than a computed 7 is what distinguishes correct
    // precedence from an accident of these particular numbers.
    let Some(ArrayLength::Counted(Expression::Binary {
        operator: BinaryOperator::Add,
        right,
        ..
    })) = &placement.array_length
    else {
        panic!("expected addition at the root");
    };
    assert!(matches!(
        **right,
        Expression::Binary {
            operator: BinaryOperator::Multiply,
            ..
        }
    ));
}

#[test]
fn malformed_sources_report_stable_codes() {
    for (source, code) in [
        ("struct S { u8 a; ", "pattern.parse.unexpected_token"),
        ("struct S { u8 a }", "pattern.parse.expected_symbol"),
        ("struct { u8 a; };", "pattern.parse.expected_identifier"),
        ("u8 a @ ;", "pattern.parse.unexpected_token"),
        ("struct S { 4 a; };", "pattern.parse.expected_type"),
        ("u8 a[0] @ 0;", "pattern.parse.invalid_array_length"),
        ("bitfield B { a : 0; };", "pattern.parse.invalid_bit_width"),
        ("bitfield B { a : 65; };", "pattern.parse.invalid_bit_width"),
        // A width that needs a field cannot be folded, so it is rejected where
        // it is written rather than at evaluation time.
        (
            "bitfield B { a : other; };",
            "pattern.parse.invalid_bit_width",
        ),
        ("union U { u8 a; ", "pattern.parse.unexpected_token"),
        ("struct S { if (1) { u8 a; }", "pattern.parse.unexpected_token"),
        ("struct S { u8 a; }; union S { u8 b; };", "pattern.parse.duplicate_declaration"),
        (
            "struct S { u8 a; }; struct S { u8 b; };",
            "pattern.parse.duplicate_declaration",
        ),
        (
            "struct S { u8 a; }; /* open",
            "pattern.lex.unterminated_comment",
        ),
        // A radix prefix with no digits fails in the lexer, before the parser
        // ever asks for a value.
        ("u8 a @ 0x; ", "pattern.lex.invalid_number"),
        ("struct S { u8 a; } #", "pattern.lex.unexpected_character"),
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

