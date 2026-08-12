//! Base85 conformance vectors pinned against `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

const Z85: &str = "0-9a-zA-Z.\\-:+=^!/*?&<>()[]{}@%$#";
const IPV6: &str = "0-9A-Za-z!#$%&()*+\\-;<=>?@^_`{|}~";

#[test]
fn base85_round_trips_standard_vectors() {
    let encoded = support::run(
        "encoding.base85.encode@1",
        Arguments::new(),
        Value::Bytes(b"hello world".to_vec()),
    );
    assert_eq!(support::output_text(encoded), "BOu!rD]j7BEbo7");

    let decoded = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("BOu!rD]j7BEbo7"),
    );
    assert_eq!(support::output_bytes(decoded), b"hello world");
}

#[test]
fn base85_wraps_and_unwraps_adobe_delimiters() {
    let arguments = Arguments::from([
        ("alphabet".into(), ArgumentValue::Text("!-u".into())),
        ("include_delimiter".into(), ArgumentValue::Boolean(true)),
    ]);
    let encoded = support::run(
        "encoding.base85.encode@1",
        arguments,
        Value::Bytes(b"hello world".to_vec()),
    );
    assert_eq!(support::output_text(encoded), "<~BOu!rD]j7BEbo7~>");

    // A line feed between the markers defeats the reference's first
    // anchored match, and the second pass (after noise removal) succeeds.
    let decoded = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("<~BOu!rD]j7\nBEbo7~>"),
    );
    assert_eq!(support::output_bytes(decoded), b"hello world");
}

#[test]
fn base85_compresses_zero_groups_in_the_standard_alphabet() {
    for (input, encoded) in [
        (vec![0x00, 0x00, 0x00, 0x00], "z"),
        (vec![0x00], "z"),
        (vec![0x00, 0x00, 0x00, 0x00, 0x00], "zz"),
        (vec![0xff, 0xff, 0xff, 0xff], "s8W-!"),
    ] {
        let result = support::run(
            "encoding.base85.encode@1",
            Arguments::new(),
            Value::Bytes(input),
        );
        assert_eq!(support::output_text(result), encoded);
    }

    let decoded = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("z"),
    );
    assert_eq!(support::output_bytes(decoded), [0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn base85_supports_z85_and_ipv6_alphabets() {
    let arguments = support::argument("alphabet", ArgumentValue::Text(Z85.into()));
    let result = support::run(
        "encoding.base85.encode@1",
        arguments.clone(),
        Value::Bytes(b"hello world".to_vec()),
    );
    assert_eq!(support::output_text(result), "xK#0@zY<mxA+]m");

    let result = support::run(
        "encoding.base85.encode@1",
        arguments,
        Value::Bytes(vec![0x00, 0x00, 0x00, 0x00]),
    );
    assert_eq!(support::output_text(result), "00000");

    let arguments = support::argument("alphabet", ArgumentValue::Text(IPV6.into()));
    let result = support::run(
        "encoding.base85.encode@1",
        arguments,
        Value::Bytes(b"hello".to_vec()),
    );
    assert_eq!(support::output_text(result), "Xk~0{Zv");
}

#[test]
fn base85_replicates_reference_block_arithmetic_wrapping() {
    // "uuuuu" exceeds 2^32 and wraps through the reference's 32-bit shifts.
    let result = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("uuuuu"),
    );
    assert_eq!(support::output_bytes(result), [0x08, 0x78, 0x0e, 0xc4]);

    // A zero-group symbol inside a block keeps its raw -1 digit and drives
    // the block negative before wrapping.
    let result = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("!!z!!"),
    );
    assert_eq!(support::output_bytes(result), [0xff, 0xff, 0xe3, 0xc7]);
}

#[test]
fn base85_handles_partial_and_dangling_groups_like_the_reference() {
    let result = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("BOu"),
    );
    assert_eq!(support::output_bytes(result), [0x68, 0x65]);

    // A lone fifteenth symbol re-shapes the final block instead of failing.
    let result = support::run(
        "encoding.base85.decode@1",
        Arguments::new(),
        support::text("BOu!rD]j7BEbo7B"),
    );
    assert_eq!(
        support::output_bytes(result),
        [
            0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x63, 0xde
        ]
    );
}

#[test]
fn base85_honors_the_zero_group_character_argument() {
    let arguments = Arguments::from([
        ("alphabet".into(), ArgumentValue::Text("!-u".into())),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
        (
            "zero_group_character".into(),
            ArgumentValue::Text("y".into()),
        ),
    ]);
    let result = support::run("encoding.base85.decode@1", arguments, support::text("y"));
    assert_eq!(support::output_bytes(result), [0x00, 0x00, 0x00, 0x00]);

    let conflicting = Arguments::from([
        ("alphabet".into(), ArgumentValue::Text("!-u".into())),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
        (
            "zero_group_character".into(),
            ArgumentValue::Text("u".into()),
        ),
    ]);
    let error = support::run_with_budget(
        "encoding.base85.decode@1",
        conflicting,
        support::text("!!!!!"),
        support::budget(),
    )
    .expect_err("zero-group characters inside the alphabet must fail");
    assert_eq!(error.code(), "encoding.base85.zero_character_conflict");
}

#[test]
fn base85_rejects_foreign_characters_without_removal() {
    let strict = Arguments::from([
        ("alphabet".into(), ArgumentValue::Text("!-u".into())),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(false)),
        (
            "zero_group_character".into(),
            ArgumentValue::Text("z".into()),
        ),
    ]);
    let error = support::run_with_budget(
        "encoding.base85.decode@1",
        strict,
        support::text("BOu!~"),
        support::budget(),
    )
    .expect_err("unfiltered invalid characters must fail");
    assert_eq!(error.code(), "encoding.base85.invalid_character");
}
