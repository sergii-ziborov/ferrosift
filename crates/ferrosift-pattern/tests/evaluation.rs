//! Evaluation vectors: value decoding, layout offsets, and bounded failure.

use ferrosift_pattern::{EvalOptions, Node, NodeValue, PatternError, evaluate, parse};

fn run(source: &str, data: &[u8]) -> Vec<Node> {
    let pattern = parse(source).unwrap_or_else(|error| panic!("parse failed: {error}"));
    evaluate(&pattern, data, &EvalOptions::default())
        .unwrap_or_else(|error| panic!("evaluation failed: {error}"))
}

fn run_with(source: &str, data: &[u8], options: &EvalOptions) -> Result<Vec<Node>, PatternError> {
    let pattern = parse(source).unwrap_or_else(|error| panic!("parse failed: {error}"));
    evaluate(&pattern, data, options)
}

fn reject(source: &str, data: &[u8]) -> PatternError {
    run_with(source, data, &EvalOptions::default()).expect_err("expected a rejection")
}

#[test]
fn struct_fields_carry_their_exact_offsets_and_sizes() {
    let nodes = run(
        "struct Header { be u16 magic; u8 version; };
         Header header @ 0x00;",
        &[0xca, 0xfe, 0x03],
    );

    assert_eq!(nodes.len(), 1);
    let header = &nodes[0];
    assert_eq!(header.type_name, "Header");
    assert_eq!((header.offset, header.size), (0, 3));

    let magic = header.child("magic").expect("field");
    assert_eq!(magic.value, NodeValue::Unsigned(0xcafe));
    assert_eq!((magic.offset, magic.size), (0, 2));

    let version = header.child("version").expect("field");
    assert_eq!(version.value, NodeValue::Unsigned(3));
    assert_eq!((version.offset, version.size), (2, 1));
}

#[test]
fn endianness_defaults_to_little_and_prefixes_override_it() {
    let nodes = run(
        "u16 little @ 0; be u16 big @ 0; le u16 explicit @ 0;",
        &[0x34, 0x12],
    );
    assert_eq!(nodes[0].value, NodeValue::Unsigned(0x1234));
    assert_eq!(nodes[1].value, NodeValue::Unsigned(0x3412));
    assert_eq!(nodes[2].value, NodeValue::Unsigned(0x1234));
}

#[test]
fn an_endian_prefix_on_a_struct_reaches_fields_without_their_own() {
    let nodes = run(
        "struct Pair { u16 inherited; le u16 pinned; };
         be Pair pair @ 0;",
        &[0x12, 0x34, 0x56, 0x78],
    );
    let pair = &nodes[0];
    assert_eq!(
        pair.child("inherited").expect("field").value,
        NodeValue::Unsigned(0x1234)
    );
    assert_eq!(
        pair.child("pinned").expect("field").value,
        NodeValue::Unsigned(0x7856)
    );
}

#[test]
fn signed_values_are_sign_extended_from_their_width() {
    let nodes = run(
        "s8 byte @ 0; be s16 word @ 1; be s24 triple @ 3;",
        &[0xff, 0xff, 0xfe, 0xff, 0xff, 0xfd],
    );
    assert_eq!(nodes[0].value, NodeValue::Signed(-1));
    assert_eq!(nodes[1].value, NodeValue::Signed(-2));
    assert_eq!(nodes[2].value, NodeValue::Signed(-3));
}

#[test]
fn arrays_expand_into_indexed_children() {
    let nodes = run("u8 bytes[4] @ 0;", &[10, 20, 30, 40]);
    let array = &nodes[0];

    assert_eq!(array.type_name, "u8[4]");
    assert_eq!((array.offset, array.size), (0, 4));
    let children = array.children();
    assert_eq!(children.len(), 4);
    assert_eq!(children[0].name, "bytes[0]");
    assert_eq!(children[3].value, NodeValue::Unsigned(40));
    assert_eq!((children[3].offset, children[3].size), (3, 1));
}

#[test]
fn nested_structs_lay_out_consecutively() {
    let nodes = run(
        "struct Inner { u8 a; u8 b; };
         struct Outer { Inner first; Inner second; };
         Outer outer @ 0;",
        &[1, 2, 3, 4],
    );
    let outer = &nodes[0];
    assert_eq!((outer.offset, outer.size), (0, 4));

    let second = outer.child("second").expect("field");
    assert_eq!((second.offset, second.size), (2, 2));
    assert_eq!(
        second.child("b").expect("field").value,
        NodeValue::Unsigned(4)
    );
    assert_eq!(second.child("b").expect("field").offset, 3);
}

#[test]
fn enum_values_resolve_to_names_when_declared() {
    let nodes = run(
        "enum Kind : u8 { Zero, One, Five = 5 };
         Kind known @ 0;
         Kind unknown @ 1;",
        &[5, 9],
    );
    assert_eq!(
        nodes[0].value,
        NodeValue::Enumerator {
            name: Some("Five".into()),
            value: 5,
        }
    );
    assert_eq!(nodes[0].type_name, "Kind");
    assert_eq!(
        nodes[1].value,
        NodeValue::Enumerator {
            name: None,
            value: 9,
        }
    );
}

#[test]
fn bitfield_members_unpack_most_significant_first() {
    let nodes = run(
        "bitfield Nibbles { high : 4; low : 4; };
         Nibbles byte @ 0;",
        &[0xab],
    );
    let byte = &nodes[0];
    assert_eq!((byte.offset, byte.size), (0, 1));
    assert_eq!(
        byte.child("high").expect("member").value,
        NodeValue::Unsigned(0xa)
    );
    assert_eq!(
        byte.child("low").expect("member").value,
        NodeValue::Unsigned(0xb)
    );
}

#[test]
fn bitfields_round_their_storage_up_to_whole_bytes() {
    // Twelve declared bits occupy two bytes, read most significant first.
    let nodes = run(
        "bitfield Packed { first : 4; second : 8; };
         Packed packed @ 0;",
        &[0xab, 0xcd],
    );
    let packed = &nodes[0];
    assert_eq!(packed.size, 2);
    assert_eq!(
        packed.child("first").expect("member").value,
        NodeValue::Unsigned(0xa)
    );
    assert_eq!(
        packed.child("second").expect("member").value,
        NodeValue::Unsigned(0xbc)
    );
}

#[test]
fn aliases_resolve_and_keep_their_own_name() {
    let nodes = run(
        "using Word = be u16;
         Word word @ 0;",
        &[0x12, 0x34],
    );
    assert_eq!(nodes[0].value, NodeValue::Unsigned(0x1234));
    assert_eq!(nodes[0].type_name, "Word");
}

#[test]
fn floating_point_bool_and_char_decode() {
    let mut data = Vec::new();
    data.extend_from_slice(&1.0_f32.to_le_bytes());
    data.extend_from_slice(&(-2.5_f64).to_le_bytes());
    data.extend_from_slice(&[1, 0, b'A']);

    let nodes = run(
        "float single @ 0; double wide @ 4; bool yes @ 12; bool no @ 13; char letter @ 14;",
        &data,
    );
    assert_eq!(nodes[0].value, NodeValue::Float(1.0));
    assert_eq!(nodes[1].value, NodeValue::Double(-2.5));
    assert_eq!(nodes[2].value, NodeValue::Bool(true));
    assert_eq!(nodes[3].value, NodeValue::Bool(false));
    assert_eq!(nodes[4].value, NodeValue::Char('A'));
}

#[test]
fn placements_read_from_their_declared_address() {
    let nodes = run("u8 third @ 2;", &[0, 0, 7]);
    assert_eq!(nodes[0].value, NodeValue::Unsigned(7));
    assert_eq!(nodes[0].offset, 2);
}

#[test]
fn reads_past_the_data_fail_instead_of_inventing_bytes() {
    for (source, data) in [
        ("u32 value @ 0;", &[1_u8, 2][..]),
        ("u8 value @ 4;", &[1, 2][..]),
        ("u8 values[3] @ 1;", &[1, 2][..]),
    ] {
        let error = reject(source, data);
        assert_eq!(
            error.code(),
            "pattern.eval.out_of_bounds",
            "source: {source}"
        );
    }
}

#[test]
fn undeclared_types_are_rejected() {
    let error = reject("Missing value @ 0;", &[0; 8]);
    assert_eq!(error.code(), "pattern.eval.unknown_type");
}

#[test]
fn a_placed_variable_cannot_be_used_as_a_type() {
    let error = reject("u8 first @ 0; first second @ 1;", &[1, 2]);
    assert_eq!(error.code(), "pattern.eval.unknown_type");
}

#[test]
fn oversized_patterns_stop_at_the_node_budget() {
    let options = EvalOptions {
        max_nodes: 8,
        ..EvalOptions::default()
    };
    let error = run_with("u8 many[64] @ 0;", &[0; 64], &options)
        .expect_err("the budget must stop evaluation");
    assert_eq!(error.code(), "pattern.eval.node_budget_exceeded");
}

#[test]
fn deep_nesting_stops_at_the_depth_limit() {
    let options = EvalOptions {
        max_depth: 1,
        ..EvalOptions::default()
    };
    let error = run_with(
        "struct A { u8 a; };
         struct B { A inner; };
         struct C { B inner; };
         C value @ 0;",
        &[1],
        &options,
    )
    .expect_err("the depth limit must stop evaluation");
    assert_eq!(error.code(), "pattern.eval.depth_exceeded");
}

// --- expressions, conditionals, unions, padding -------------------------

#[test]
fn an_array_length_may_be_a_field_read_before_it() {
    // The whole point of the expression layer: one pattern describes a family
    // of files rather than one file. The same source is run against two
    // different counts.
    let source = "struct List { u8 count; u16 items[count]; };
                  List list @ 0;";

    let nodes = run(source, &[2, 0x11, 0x00, 0x22, 0x00]);
    let items = nodes[0].child("items").expect("field");
    assert_eq!(items.children().len(), 2);
    assert_eq!((items.offset, items.size), (1, 4));

    let nodes = run(source, &[3, 1, 0, 2, 0, 3, 0]);
    assert_eq!(nodes[0].child("items").expect("field").children().len(), 3);
}

#[test]
fn arithmetic_and_precedence_hold_at_evaluation_time() {
    let nodes = run(
        "struct S { u8 base; u8 tail[base * 2 + 1]; };
         S s @ 0;",
        &[2, 9, 9, 9, 9, 9],
    );
    // 2 * 2 + 1, not 2 * 3: multiplication binds first.
    assert_eq!(nodes[0].child("tail").expect("field").children().len(), 5);
}

#[test]
fn a_conditional_chooses_which_fields_exist() {
    let source = "struct S {
                      u8 kind;
                      if (kind == 1) { be u16 word; }
                      else { u8 byte; }
                  };
                  S s @ 0;";

    let taken = run(source, &[1, 0xab, 0xcd]);
    assert!(taken[0].child("word").is_some());
    assert!(taken[0].child("byte").is_none());
    assert_eq!(taken[0].size, 3);

    let other = run(source, &[0, 0xab]);
    assert!(other[0].child("word").is_none());
    assert_eq!(other[0].child("byte").expect("field").value, NodeValue::Unsigned(0xab));
    assert_eq!(other[0].size, 2);
}

#[test]
fn an_else_if_chain_picks_exactly_one_arm() {
    let source = "struct S {
                      u8 kind;
                      if (kind == 1) { u8 a; }
                      else if (kind == 2) { u8 b; }
                      else { u8 c; }
                  };
                  S s @ 0;";

    for (kind, expected) in [(1, "a"), (2, "b"), (7, "c")] {
        let nodes = run(source, &[kind, 0x55]);
        let present: Vec<_> = nodes[0]
            .children()
            .iter()
            .map(|child| child.name.as_str())
            .filter(|name| *name != "kind")
            .collect();
        assert_eq!(present, [expected], "kind {kind}");
    }
}

#[test]
fn union_members_share_one_address_and_the_widest_size() {
    let nodes = run(
        "union Value { be u32 word; u8 bytes[4]; be u16 half; };
         Value value @ 0;",
        &[0xde, 0xad, 0xbe, 0xef],
    );

    let value = &nodes[0];
    assert_eq!((value.offset, value.size), (0, 4));
    for name in ["word", "bytes", "half"] {
        assert_eq!(value.child(name).expect("member").offset, 0, "{name}");
    }
    assert_eq!(
        value.child("word").expect("member").value,
        NodeValue::Unsigned(0xdead_beef)
    );
    assert_eq!(
        value.child("half").expect("member").value,
        NodeValue::Unsigned(0xdead)
    );
}

#[test]
fn padding_advances_the_cursor_without_producing_a_field() {
    let nodes = run(
        "struct S { u8 first; padding[3]; u8 last; };
         S s @ 0;",
        &[1, 0, 0, 0, 9],
    );

    let s = &nodes[0];
    assert_eq!(s.children().len(), 2);
    assert_eq!(s.child("last").expect("field").offset, 4);
    assert_eq!(s.size, 5);
}

#[test]
fn a_while_sized_array_stops_when_its_test_fails() {
    let nodes = run(
        "struct S { u8 items[while($ < 4)]; };
         S s @ 0;",
        &[1, 2, 3, 4, 5, 6],
    );
    let items = nodes[0].child("items").expect("field");
    assert_eq!(items.children().len(), 4);
    assert_eq!(items.size, 4);
}

#[test]
fn sizeof_reads_builtin_widths_and_the_span_a_field_occupied() {
    let nodes = run(
        "struct Header { u8 count; u8 items[count]; };
         Header header @ 0;
         u8 body[sizeof(u16)] @ sizeof(header);",
        &[2, 0xaa, 0xbb, 0xcc, 0xdd],
    );

    // The header occupied three bytes because `count` was 2, so the body
    // starts at 3 -- an address no literal could have expressed.
    let body = &nodes[1];
    assert_eq!((body.offset, body.size), (3, 2));
}

#[test]
fn a_dotted_path_reaches_into_a_nested_field() {
    let nodes = run(
        "struct Inner { u8 length; };
         struct Outer { Inner meta; u8 data[meta.length]; };
         Outer outer @ 0;",
        &[3, 7, 7, 7],
    );
    assert_eq!(nodes[0].child("data").expect("field").children().len(), 3);
}

#[test]
fn a_conditional_only_evaluates_the_branch_it_takes() {
    // The false arm divides by a zero that the test has already excluded.
    // Evaluating both arms would fail on a pattern that is correct.
    let nodes = run(
        "struct S { u8 n; u8 items[n == 0 ? 1 : 4 / n]; };
         S s @ 0;",
        &[0, 9],
    );
    assert_eq!(nodes[0].child("items").expect("field").children().len(), 1);
}

#[test]
fn expression_failures_carry_stable_codes() {
    for (source, data, code) in [
        (
            "struct S { u8 n; u8 items[4 / n]; }; S s @ 0;",
            &[0_u8, 1, 2, 3, 4][..],
            "pattern.eval.divide_by_zero",
        ),
        (
            "struct S { u8 items[missing]; }; S s @ 0;",
            &[1, 2][..],
            "pattern.eval.unknown_field",
        ),
        (
            "struct S { u8 a; u8 items[b.c]; u8 b; }; S s @ 0;",
            &[1, 2][..],
            "pattern.eval.unknown_field",
        ),
        (
            "struct S { float f; u8 items[f]; }; S s @ 0;",
            &[0, 0, 0, 0, 1][..],
            "pattern.eval.not_a_number",
        ),
    ] {
        let error = reject(source, data);
        assert_eq!(error.code(), code, "source: {source}");
    }
}

#[test]
fn a_field_cannot_refer_to_one_declared_after_it() {
    // Not a restriction this crate adds: the later field's bytes have not been
    // read, so its value does not exist yet.
    let error = reject(
        "struct S { u8 items[count]; u8 count; }; S s @ 0;",
        &[1, 2, 3],
    );
    assert_eq!(error.code(), "pattern.eval.unknown_field");
}

#[test]
fn a_while_array_that_never_advances_is_stopped_by_the_node_budget() {
    let options = EvalOptions {
        max_nodes: 64,
        ..EvalOptions::default()
    };
    let error = run_with(
        "struct Empty { padding[0]; };
         struct S { Empty items[while(true)]; };
         S s @ 0;",
        &[0; 8],
        &options,
    )
    .expect_err("expected the budget to stop it");
    assert_eq!(error.code(), "pattern.eval.node_budget_exceeded");
}
