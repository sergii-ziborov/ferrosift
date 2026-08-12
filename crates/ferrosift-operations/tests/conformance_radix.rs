//! Binary, decimal, and octal conformance vectors pinned against
//! `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn delimited(delimiter: &str) -> Arguments {
    support::argument("delimiter", ArgumentValue::Text(delimiter.into()))
}

#[test]
fn binary_encodes_every_delimiter_and_width() {
    let input = Value::Bytes(vec![0x0a, 0x14, 0x1e]);
    assert_eq!(
        support::output_text(support::run(
            "encoding.binary.encode@1",
            Arguments::new(),
            input.clone()
        )),
        "00001010 00010100 00011110"
    );
    assert_eq!(
        support::output_text(support::run(
            "encoding.binary.encode@1",
            delimited("None"),
            input
        )),
        "000010100001010000011110"
    );
    assert_eq!(
        support::output_text(support::run(
            "encoding.binary.encode@1",
            delimited("CRLF"),
            Value::Bytes(vec![0x0a, 0x14])
        )),
        "00001010\r\n00010100"
    );

    // Width never truncates: 0xff keeps all eight digits at width four.
    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("byte_length".into(), ArgumentValue::Integer(4)),
    ]);
    assert_eq!(
        support::output_text(support::run(
            "encoding.binary.encode@1",
            arguments,
            Value::Bytes(vec![0xff, 0x01])
        )),
        "11111111 0001"
    );

    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("byte_length".into(), ArgumentValue::Integer(16)),
    ]);
    assert_eq!(
        support::output_text(support::run(
            "encoding.binary.encode@1",
            arguments,
            Value::Bytes(vec![0xff])
        )),
        "0000000011111111"
    );
}

#[test]
fn binary_decoding_strips_whitespace_like_the_reference() {
    // The reference's Space and None delimiters strip every whitespace
    // character, not just spaces.
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.binary.decode@1",
            Arguments::new(),
            support::text("00001010\t00010100\n00011110")
        )),
        [0x0a, 0x14, 0x1e]
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.binary.decode@1",
            delimited("None"),
            support::text("00001010 00010100")
        )),
        [0x0a, 0x14]
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.binary.decode@1",
            delimited("Colon"),
            support::text("00001010:00010100")
        )),
        [0x0a, 0x14]
    );
}

#[test]
fn binary_decoding_supports_narrow_widths_and_partial_tails() {
    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("byte_length".into(), ArgumentValue::Integer(3)),
    ]);
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.binary.decode@1",
            arguments,
            support::text("010011")
        )),
        [0x02, 0x03]
    );

    assert_eq!(
        support::output_bytes(support::run(
            "encoding.binary.decode@1",
            Arguments::new(),
            support::text("0000101001")
        )),
        [0x0a, 0x01]
    );
}

#[test]
fn binary_rejects_values_the_reference_cannot_represent() {
    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("byte_length".into(), ArgumentValue::Integer(12)),
    ]);
    let error = support::run_with_budget(
        "encoding.binary.decode@1",
        arguments,
        support::text("111111111111"),
        support::budget(),
    )
    .expect_err("values above 255 must fail");
    assert_eq!(error.code(), "encoding.binary.value_out_of_range");

    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("byte_length".into(), ArgumentValue::Integer(0)),
    ]);
    let error = support::run_with_budget(
        "encoding.binary.decode@1",
        arguments,
        support::text("1"),
        support::budget(),
    )
    .expect_err("zero byte length must fail");
    assert_eq!(error.code(), "encoding.binary.invalid_byte_length");
}

#[test]
fn decimal_encodes_delimiters_and_signed_values() {
    assert_eq!(
        support::output_text(support::run(
            "encoding.decimal.encode@1",
            Arguments::new(),
            Value::Bytes(b"Hello".to_vec())
        )),
        "72 101 108 108 111"
    );
    assert_eq!(
        support::output_text(support::run(
            "encoding.decimal.encode@1",
            delimited("Comma"),
            Value::Bytes(vec![0x00, 0xff, 0x10])
        )),
        "0,255,16"
    );

    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("support_signed".into(), ArgumentValue::Boolean(true)),
    ]);
    assert_eq!(
        support::output_text(support::run(
            "encoding.decimal.encode@1",
            arguments,
            Value::Bytes(vec![0x80, 0xff, 0x7f, 0x00])
        )),
        "-128 -1 127 0"
    );
}

#[test]
fn decimal_decodes_every_delimiter_including_auto() {
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.decimal.decode@1",
            Arguments::new(),
            support::text("72 101 108 108 111")
        )),
        b"Hello"
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.decimal.decode@1",
            delimited("Auto"),
            support::text("72, 101; 108\n111")
        )),
        [72, 101, 108, 111]
    );

    // Auto keeps dashes inside tokens; the reference's parseInt reads the
    // leading digits and ignores the rest.
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.decimal.decode@1",
            delimited("Auto"),
            support::text("1-2 7")
        )),
        [0x01, 0x07]
    );

    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Space".into())),
        ("support_signed".into(), ArgumentValue::Boolean(true)),
    ]);
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.decimal.decode@1",
            arguments,
            support::text("-128 -1 255 127")
        )),
        [0x80, 0xff, 0xff, 0x7f]
    );
}

#[test]
fn decimal_rejects_values_outside_the_byte_range() {
    let error = support::run_with_budget(
        "encoding.decimal.decode@1",
        Arguments::new(),
        support::text("256"),
        support::budget(),
    )
    .expect_err("values above 255 must fail");
    assert_eq!(error.code(), "encoding.decimal.value_out_of_range");

    let error = support::run_with_budget(
        "encoding.decimal.decode@1",
        Arguments::new(),
        support::text("-1"),
        support::budget(),
    )
    .expect_err("negative unsigned values must fail");
    assert_eq!(error.code(), "encoding.decimal.value_out_of_range");
}

#[test]
fn octal_round_trips_reference_vectors() {
    assert_eq!(
        support::output_text(support::run(
            "encoding.octal.encode@1",
            Arguments::new(),
            Value::Bytes(vec![0x48, 0x65, 0x6c, 0x6c])
        )),
        "110 145 154 154"
    );
    assert_eq!(
        support::output_text(support::run(
            "encoding.octal.encode@1",
            delimited("Comma"),
            Value::Bytes(vec![0x00, 0xff, 0x07])
        )),
        "0,377,7"
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.octal.decode@1",
            delimited("Comma"),
            support::text("316,223")
        )),
        [0xce, 0x93]
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.octal.decode@1",
            Arguments::new(),
            support::text("0")
        )),
        [0x00]
    );
}

#[test]
fn octal_replicates_reference_empty_token_coercion() {
    // Splitting on the literal delimiter keeps empty tokens, which the
    // reference's parseInt coerces to NaN and the byte pipeline to zero.
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.octal.decode@1",
            Arguments::new(),
            support::text("40  40")
        )),
        [0x20, 0x00, 0x20]
    );
}

#[test]
fn octal_rejects_values_outside_the_byte_range() {
    let error = support::run_with_budget(
        "encoding.octal.decode@1",
        Arguments::new(),
        support::text("400"),
        support::budget(),
    )
    .expect_err("values above 255 must fail");
    assert_eq!(error.code(), "encoding.octal.value_out_of_range");
}
