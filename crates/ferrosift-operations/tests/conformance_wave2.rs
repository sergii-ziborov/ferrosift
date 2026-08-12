//! Conformance for XOR, hexdump, gunzip, slice, head, and find/replace.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn xor_matches_standard_cascade_and_null_preserving_vectors() {
    let key = |hex: &str, scheme: &str, null_preserving: bool| {
        Arguments::from([
            (
                "key".into(),
                ArgumentValue::Map(Arguments::from([
                    ("option".into(), ArgumentValue::Text("Hex".into())),
                    ("string".into(), ArgumentValue::Text(hex.into())),
                ])),
            ),
            ("scheme".into(), ArgumentValue::Text(scheme.into())),
            (
                "null_preserving".into(),
                ArgumentValue::Boolean(null_preserving),
            ),
        ])
    };

    assert_eq!(
        support::output_bytes(support::run(
            "logic.xor@1",
            key("0f", "Standard", false),
            Value::Bytes(b"Hello".to_vec()),
        )),
        [0x47, 0x6a, 0x63, 0x63, 0x60]
    );
    assert_eq!(
        support::output_bytes(support::run(
            "logic.xor@1",
            key("01", "Standard", true),
            Value::Bytes(vec![0, 1, 0, 2]),
        )),
        [0, 1, 0, 3]
    );
    assert_eq!(
        support::output_bytes(support::run(
            "logic.xor@1",
            key("ff", "Cascade", false),
            Value::Bytes(vec![1, 2, 3, 4]),
        )),
        [3, 1, 7, 4]
    );
}

#[test]
fn hexdump_round_trips_and_honors_width_flags() {
    let encoded = support::output_text(support::run(
        "encoding.hexdump.encode@1",
        Arguments::from([
            ("width".into(), ArgumentValue::Integer(16)),
            ("upper_case_hex".into(), ArgumentValue::Boolean(false)),
            ("include_final_length".into(), ArgumentValue::Boolean(false)),
            ("unix_format".into(), ArgumentValue::Boolean(false)),
        ]),
        Value::Bytes(b"FerroSift".to_vec()),
    ));
    assert!(encoded.contains("46 65 72 72 6f 53 69 66 74"));
    assert!(encoded.contains("|FerroSift|"));

    let decoded = support::output_bytes(support::run(
        "encoding.hexdump.decode@1",
        Arguments::new(),
        support::text(&encoded),
    ));
    assert_eq!(decoded, b"FerroSift");

    let upper = support::output_text(support::run(
        "encoding.hexdump.encode@1",
        Arguments::from([
            ("width".into(), ArgumentValue::Integer(8)),
            ("upper_case_hex".into(), ArgumentValue::Boolean(true)),
            ("include_final_length".into(), ArgumentValue::Boolean(true)),
            ("unix_format".into(), ArgumentValue::Boolean(false)),
        ]),
        Value::Bytes(b"ABCDEFGH".to_vec()),
    ));
    assert!(upper.contains("41 42 43 44 45 46 47 48"));
    assert!(upper.contains("00000008"));
}

#[test]
fn take_drop_and_head_slice_like_the_reference() {
    assert_eq!(
        support::output_bytes(support::run(
            "data.take_bytes@1",
            Arguments::from([
                ("start".into(), ArgumentValue::Integer(2)),
                ("length".into(), ArgumentValue::Integer(3)),
                ("apply_to_each_line".into(), ArgumentValue::Boolean(false)),
            ]),
            Value::Bytes(b"abcdefgh".to_vec()),
        )),
        b"cde"
    );
    assert_eq!(
        support::output_bytes(support::run(
            "data.drop_bytes@1",
            Arguments::from([
                ("start".into(), ArgumentValue::Integer(2)),
                ("length".into(), ArgumentValue::Integer(3)),
                ("apply_to_each_line".into(), ArgumentValue::Boolean(false)),
            ]),
            Value::Bytes(b"abcdefgh".to_vec()),
        )),
        b"abfgh"
    );
    assert_eq!(
        support::output_text(support::run(
            "data.head@1",
            Arguments::from([
                ("delimiter".into(), ArgumentValue::Text("Line feed".into())),
                ("number".into(), ArgumentValue::Integer(2)),
            ]),
            support::text("a\nb\nc\nd"),
        )),
        "a\nb"
    );
}

#[test]
fn find_replace_supports_simple_and_extended_modes() {
    let simple = support::output_text(support::run(
        "text.find_replace@1",
        Arguments::from([
            (
                "find".into(),
                ArgumentValue::Map(Arguments::from([
                    ("option".into(), ArgumentValue::Text("Simple string".into())),
                    ("string".into(), ArgumentValue::Text("foo".into())),
                ])),
            ),
            ("replace".into(), ArgumentValue::Text("x".into())),
            ("global_match".into(), ArgumentValue::Boolean(true)),
            ("case_insensitive".into(), ArgumentValue::Boolean(false)),
            ("multiline_matching".into(), ArgumentValue::Boolean(true)),
            ("dot_matches_all".into(), ArgumentValue::Boolean(false)),
        ]),
        support::text("foo bar foo"),
    ));
    assert_eq!(simple, "x bar x");

    let extended = support::output_text(support::run(
        "text.find_replace@1",
        Arguments::from([
            (
                "find".into(),
                ArgumentValue::Map(Arguments::from([
                    (
                        "option".into(),
                        ArgumentValue::Text("Extended (\\n, \\t, \\x...)".into()),
                    ),
                    ("string".into(), ArgumentValue::Text("\\t".into())),
                ])),
            ),
            ("replace".into(), ArgumentValue::Text("-".into())),
            ("global_match".into(), ArgumentValue::Boolean(true)),
            ("case_insensitive".into(), ArgumentValue::Boolean(false)),
            ("multiline_matching".into(), ArgumentValue::Boolean(true)),
            ("dot_matches_all".into(), ArgumentValue::Boolean(false)),
        ]),
        support::text("a\tb"),
    ));
    assert_eq!(extended, "a-b");
}

#[test]
fn gunzip_decompresses_reference_payload() {
    let gzip = hex::decode("1f8b080000000000000acb48cdc9c957484b2d2aca070084a9e47c0b000000")
        .expect("fixture hex");
    let output = support::output_bytes(support::run(
        "compression.gunzip@1",
        Arguments::new(),
        Value::Bytes(gzip),
    ));
    assert_eq!(output, b"hello ferro");
}

mod hex {
    pub fn decode(value: &str) -> Result<Vec<u8>, ()> {
        if !value.len().is_multiple_of(2) {
            return Err(());
        }
        let mut output = Vec::with_capacity(value.len() / 2);
        for chunk in value.as_bytes().chunks(2) {
            let text = core::str::from_utf8(chunk).map_err(|_| ())?;
            output.push(u8::from_str_radix(text, 16).map_err(|_| ())?);
        }
        Ok(output)
    }
}
