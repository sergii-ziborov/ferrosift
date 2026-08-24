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
