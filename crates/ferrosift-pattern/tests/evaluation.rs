//! Evaluation vectors: value decoding, layout offsets, and bounded failure.

use ferrosift_pattern::{Builtin, EvalOptions, Node, NodeValue, PatternError, evaluate, parse};

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
fn an_array_of_scalars_is_kept_as_the_bytes_it_came_from() {
    // These used to be four `Node`s, each carrying the words `bytes[2]` and
    // `u8` on the heap. A node costs something over a hundred bytes and a `u8`
    // costs one, so an array of a million of them cost a hundred megabytes to
    // say what a megabyte already said.
    let nodes = run("u8 bytes[4] @ 0;", &[10, 20, 30, 40]);
    let array = &nodes[0];

    assert_eq!(array.type_name, "u8[4]");
    assert_eq!((array.offset, array.size), (0, 4));
    assert_eq!(array.element_count(), 4);
    assert_eq!(array.element(0), Some(NodeValue::Unsigned(10)));
    assert_eq!(array.element(3), Some(NodeValue::Unsigned(40)));
    assert_eq!(array.element(4), None);
    // No children, and that is the visible half of the change.
    assert!(array.children().is_empty());

    let NodeValue::Scalars(scalars) = &array.value else {
        panic!("an array of a built-in scalar keeps its bytes");
    };
    assert_eq!(scalars.bytes(), &[10, 20, 30, 40]);
    assert_eq!(scalars.element_type(), Builtin::Unsigned(1));
    assert_eq!(
        scalars.iter().collect::<Vec<_>>(),
        [10, 20, 30, 40].map(NodeValue::Unsigned)
    );
}

#[test]
fn a_large_scalar_array_costs_its_bytes_rather_than_its_elements() {
    // Just under the default node budget, so this is the largest array the
    // engine will build — and it is a megabyte of storage rather than the
    // hundred-odd megabytes a million `Node`s would have been. That the test
    // runs at all is most of what it asserts.
    const COUNT: usize = 999_999;
    let data = vec![0xab_u8; COUNT];
    let nodes = run("u8 blob[999999] @ 0;", &data);

    let NodeValue::Scalars(scalars) = &nodes[0].value else {
        panic!("an array of a built-in scalar keeps its bytes");
    };
    assert_eq!(scalars.len(), COUNT);
    assert_eq!(scalars.bytes().len(), COUNT, "one byte per element");
    assert_eq!(nodes[0].size, COUNT as u64);
    assert_eq!(scalars.get(COUNT - 1), Some(NodeValue::Unsigned(0xab)));
}

#[test]
fn the_node_budget_still_bounds_a_scalar_array() {
    // Charged per element even though no node is built. The budget is the
    // caller's statement about how large a value tree they will accept, and
    // answering a bigger one because it happens to be cheap now would move
    // that boundary without being asked to.
    let data = vec![0_u8; 32];
    let options = EvalOptions {
        max_nodes: 8,
        ..EvalOptions::default()
    };
    // One for the array itself, then one per element.
    assert!(run_with("u8 x[7] @ 0;", &data, &options).is_ok());
    let error = run_with("u8 x[8] @ 0;", &data, &options)
        .expect_err("eight elements plus the array itself is nine");
    assert_eq!(error.code(), "pattern.eval.node_budget_exceeded");
}

#[test]
fn an_array_of_composites_still_expands_into_children() {
    // The compact form is only for elements that are numbers. A struct element
    // is a tree, and there is nothing to defer.
    let nodes = run(
        "struct Pair { u8 a; u8 b; };
         Pair pairs[2] @ 0;",
        &[1, 2, 3, 4],
    );
    let array = &nodes[0];

    assert_eq!(array.element_count(), 2);
    let children = array.children();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "pairs[0]");
    assert_eq!((children[1].offset, children[1].size), (2, 2));
    assert_eq!(
        children[1].child("b").expect("field").value,
        NodeValue::Unsigned(4)
    );
}

#[test]
fn an_alias_of_a_scalar_is_compact_too() {
    // `using Byte = u8;` describes the same megabyte as `u8` does, and it
    // would be a strange rule that made one of them a hundred times more
    // expensive than the other. Aliases are followed to the built-in behind
    // them, carrying whichever endianness prefix wins.
    let nodes = run(
        "using Word = be u16;
         Word words[3] @ 0;",
        &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
    );
    let array = &nodes[0];

    assert_eq!(array.element_count(), 3);
    assert!(array.children().is_empty());
    assert_eq!(array.element(0), Some(NodeValue::Unsigned(0x0102)));
    assert_eq!(array.element(2), Some(NodeValue::Unsigned(0x0506)));
    assert_eq!(array.type_name, "Word[3]");
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

/// Bit order follows byte order, which is the reference's rule.
///
/// A little-endian span — the default — gives the first member the least
/// significant bits, so `first : 4` of `0xab` is `0xb` and not `0xa`. Asking
/// for `be` puts the members in the same direction as the bytes and gives the
/// other answer. Both are in the differential fixture; this states the pair
/// side by side, which the fixture's one-case-per-file shape cannot.
#[test]
fn bit_order_follows_byte_order() {
    let little = run(
        "bitfield Nibbles { low : 4; high : 4; };
         Nibbles byte @ 0;",
        &[0xab],
    );
    let byte = &little[0];
    assert_eq!((byte.offset, byte.size), (0, 1));
    assert_eq!(
        byte.child("low").expect("member").value,
        NodeValue::Unsigned(0xb)
    );
    assert_eq!(
        byte.child("high").expect("member").value,
        NodeValue::Unsigned(0xa)
    );

    let big = run(
        "bitfield Nibbles { high : 4; low : 4; };
         struct S { be Nibbles byte; };
         S s @ 0;",
        &[0xab],
    );
    let byte = big[0].child("byte").expect("member");
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
    // Twelve declared bits occupy two bytes. Read little-endian, the span is
    // `0xcdab`: the first four bits are `0xb` and the next eight `0xda`.
    let nodes = run(
        "bitfield Packed { first : 4; second : 8; };
         Packed packed @ 0;",
        &[0xab, 0xcd],
    );
    let packed = &nodes[0];
    assert_eq!(packed.size, 2);
    assert_eq!(
        packed.child("first").expect("member").value,
        NodeValue::Unsigned(0xb)
    );
    assert_eq!(
        packed.child("second").expect("member").value,
        NodeValue::Unsigned(0xda)
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
    assert_eq!(items.element_count(), 2);
    assert_eq!((items.offset, items.size), (1, 4));

    let nodes = run(source, &[3, 1, 0, 2, 0, 3, 0]);
    assert_eq!(nodes[0].child("items").expect("field").element_count(), 3);
}

#[test]
fn arithmetic_and_precedence_hold_at_evaluation_time() {
    let nodes = run(
        "struct S { u8 base; u8 tail[base * 2 + 1]; };
         S s @ 0;",
        &[2, 9, 9, 9, 9, 9],
    );
    // 2 * 2 + 1, not 2 * 3: multiplication binds first.
    assert_eq!(nodes[0].child("tail").expect("field").element_count(), 5);
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
    assert_eq!(
        other[0].child("byte").expect("field").value,
        NodeValue::Unsigned(0xab)
    );
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
    assert_eq!(items.element_count(), 4);
    assert_eq!(items.size, 4);
}

/// A `while`-array whose element reads nothing is refused, not counted.
///
/// The condition sees `$`, and an element of zero width leaves `$` where it
/// was — so the test never changes its mind and the loop has no end of its own.
/// The node budget did stop it, after a million iterations that allocated a
/// million nodes to say nothing. Refusing on the first element that occupies no
/// bytes costs one comparison and names the actual problem.
#[test]
fn a_while_sized_array_refuses_an_element_that_reads_nothing() {
    let error = reject(
        "struct Empty {};
         struct S { Empty items[while($ < 4)]; };
         S s @ 0;",
        &[1, 2, 3, 4],
    );
    assert_eq!(error.code(), "pattern.eval.zero_width_loop");

    // Padding of nothing is the other way to occupy no bytes.
    let error = reject(
        "struct Nothing { padding[0]; };
         struct S { Nothing items[while($ < 4)]; };
         S s @ 0;",
        &[1, 2, 3, 4],
    );
    assert_eq!(error.code(), "pattern.eval.zero_width_loop");
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
    assert_eq!(nodes[0].child("data").expect("field").element_count(), 3);
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
    assert_eq!(nodes[0].child("items").expect("field").element_count(), 1);
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

/// A loop that cannot end is refused for that reason, not for running out.
///
/// The node budget still stands behind it and still stops a loop that advances
/// by some amount the data never satisfies. What it should not be is the answer
/// to a loop that was never going to end: a million iterations and a million
/// nodes to report a ceiling, where the first element already showed the
/// problem.
#[test]
fn a_while_array_that_never_advances_says_so_rather_than_running_out() {
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
    .expect_err("expected the loop to be refused");
    assert_eq!(error.code(), "pattern.eval.zero_width_loop");

    // The budget is still what stops a loop that *does* advance.
    let error = run_with(
        "struct S { u8 items[while(true)]; };
         S s @ 0;",
        &[0; 8],
        &options,
    )
    .expect_err("expected the budget to stop it");
    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
}

#[test]
fn a_nested_type_cannot_reach_the_body_that_holds_it() {
    // Documented limitation rather than a bug being hidden: expressions
    // resolve against siblings, so an inner type has no path to the outer
    // field. Pinning the code keeps the limit honest -- if scoping is ever
    // widened, this test is what says so.
    let error = reject(
        "struct Inner { u8 data[length]; };
         struct Outer { u8 length; Inner inner; };
         Outer outer @ 0;",
        &[2, 7, 7],
    );
    assert_eq!(error.code(), "pattern.eval.unknown_field");
}

#[test]
fn sizeof_a_declared_type_is_read_as_a_field_and_fails() {
    // `sizeof(Header)` looks reasonable and is not supported: a body with an
    // `if` in it has no single width, so the answer needs evaluation. It fails
    // as an unreadable field rather than silently returning something.
    let error = reject(
        "struct Header { u8 a; };
         u8 body[sizeof(Header)] @ 0;",
        &[1, 2, 3],
    );
    assert_eq!(error.code(), "pattern.eval.unknown_field");
}

#[test]
fn short_circuit_operators_do_not_evaluate_the_dead_side() {
    // The right side divides by a zero the left side has already excluded.
    let nodes = run(
        "struct S { u8 n; u8 items[(n != 0 && 8 / n > 100) ? 1 : 2]; };
         S s @ 0;",
        &[0, 9, 9],
    );
    assert_eq!(nodes[0].child("items").expect("field").element_count(), 2);
}

#[test]
fn a_computed_zero_length_yields_an_empty_array() {
    // A literal `[0]` is refused while parsing, but a computed zero cannot be:
    // its value is not known until the bytes are read. An empty array is the
    // answer rather than a failure.
    let nodes = run(
        "struct S { u8 n; u8 items[n]; u8 tail; };
         S s @ 0;",
        &[0, 0xab],
    );
    let items = nodes[0].child("items").expect("field");
    assert_eq!(items.element_count(), 0);
    assert_eq!((items.offset, items.size), (1, 0));
    // The zero-width array must not disturb what follows it.
    assert_eq!(nodes[0].child("tail").expect("field").offset, 1);
}

#[test]
fn a_union_takes_its_widest_member_even_when_declared_first() {
    let nodes = run(
        "union U { be u32 wide; u8 narrow; };
         struct S { U u; u8 after; };
         S s @ 0;",
        &[1, 2, 3, 4, 0xff],
    );
    assert_eq!(nodes[0].child("u").expect("member").size, 4);
    assert_eq!(nodes[0].child("after").expect("field").offset, 4);
}

#[test]
fn conditionals_nest_inside_a_union_and_stay_overlaid() {
    let nodes = run(
        "union U {
             u8 tag;
             if (tag == 1) { be u16 pair; }
         };
         U u @ 0;",
        &[1, 0x02],
    );
    let pair = nodes[0].child("pair").expect("member");
    assert_eq!(pair.offset, 0);
    assert_eq!(pair.value, NodeValue::Unsigned(0x0102));
    assert_eq!(nodes[0].size, 2);
}

#[test]
fn a_realistic_container_format_parses_end_to_end() {
    // The expression layer exercised the way a real pattern would use it: a
    // header whose length decides where the body starts, a record count taken
    // from the file, a per-record tag choosing between two payload shapes, and
    // a body placed after a header of no fixed size.
    //
    // None of this could be written before. Every length and address had to be
    // a literal, so a pattern matched one file rather than a format.
    let source = "
        enum Tag : u8 { Text = 1, Number = 2 };

        struct Record {
            Tag tag;
            u8 length;
            if (tag == Tag::Text) { char text[length]; }
            else { be u32 number; }
        };

        struct Header {
            be u16 magic;
            u8 name_length;
            char name[name_length];
            u8 records;
        };

        Header header @ 0;
        Record body[header.records] @ sizeof(header);
    ";

    let data = [
        // header: magic, name length 3, `abc`, two records
        0xca, 0xfe, 3, b'a', b'b', b'c', 2, // record 0: Text, length 2, `hi`
        1, 2, b'h', b'i', // record 1: Number, length 4, 42 big-endian
        2, 4, 0x00, 0x00, 0x00, 0x2a,
    ];

    let nodes = run(source, &data);

    let header = &nodes[0];
    assert_eq!(
        header.child("magic").expect("field").value,
        NodeValue::Unsigned(0xcafe)
    );
    assert_eq!(header.size, 7);

    let body = &nodes[1];
    assert_eq!(body.offset, 7);
    assert_eq!(body.children().len(), 2);

    let first = &body.children()[0];
    assert_eq!(
        first.child("tag").expect("field").value,
        NodeValue::Enumerator {
            name: Some("Text".into()),
            value: 1
        }
    );
    assert_eq!(first.child("text").expect("field").element_count(), 2);
    assert!(first.child("number").is_none());

    let second = &body.children()[1];
    assert_eq!(
        second.child("number").expect("field").value,
        NodeValue::Unsigned(42)
    );
    assert!(second.child("text").is_none());

    // Every node still carries the exact bytes it came from, which is the
    // property the whole crate exists for.
    assert_eq!((second.offset, second.size), (11, 6));
}

#[test]
fn an_enum_constant_is_qualified_by_the_enum_that_declares_it() {
    // Two enums declaring the same name is why the qualifier is required: a
    // bare `Read` would have to pick one, and which one it picked would depend
    // on what else happened to be declared beside it.
    let source = "
        enum Access : u8 { None = 0, Read = 1 };
        enum Mode : u8 { Write = 1, Read = 2 };
        struct S {
            u8 kind;
            if (kind == Mode::Read) { u8 from_mode; }
            else if (kind == Access::Read) { u8 from_access; }
        };
        S s @ 0;
    ";

    let nodes = run(source, &[2, 0xaa]);
    assert!(nodes[0].child("from_mode").is_some());

    let nodes = run(source, &[1, 0xaa]);
    assert!(nodes[0].child("from_access").is_some());
}

#[test]
fn unknown_enum_constants_report_a_stable_code() {
    for source in [
        "enum E : u8 { A = 1 }; struct S { u8 k; if (k == E::Missing) { u8 x; } }; S s @ 0;",
        "struct S { u8 k; if (k == Absent::A) { u8 x; } }; S s @ 0;",
    ] {
        let error = reject(source, &[1, 2]);
        assert_eq!(error.code(), "pattern.eval.unknown_constant", "{source}");
    }
}
