//! Base32 conformance vectors pinned against `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn base32_matches_rfc_4648_vectors() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "MY======"),
        ("fo", "MZXQ===="),
        ("foo", "MZXW6==="),
        ("foob", "MZXW6YQ="),
        ("fooba", "MZXW6YTB"),
        ("foobar", "MZXW6YTBOI======"),
    ] {
        let encoded_result = support::run(
            "encoding.base32.encode@1",
            Arguments::new(),
            Value::Bytes(plain.as_bytes().to_vec()),
        );
        assert_eq!(support::output_text(encoded_result), encoded);

        let decoded_result = support::run(
            "encoding.base32.decode@1",
            Arguments::new(),
            support::text(encoded),
        );
        assert_eq!(support::output_bytes(decoded_result), plain.as_bytes());
    }
}

#[test]
fn base32_supports_edge_bytes_and_the_hex_extended_alphabet() {
    let result = support::run(
        "encoding.base32.encode@1",
        Arguments::new(),
        Value::Bytes(vec![0x00, 0xff, 0x10, 0x7f]),
    );
    assert_eq!(support::output_text(result), "AD7RA7Y=");

    let arguments = support::argument("alphabet", ArgumentValue::Text("0-9A-V=".into()));
    let result = support::run(
        "encoding.base32.encode@1",
        arguments.clone(),
        Value::Bytes(b"foo".to_vec()),
    );
    assert_eq!(support::output_text(result), "CPNMU===");

    let result = support::run(
        "encoding.base32.decode@1",
        arguments,
        support::text("CPNMU==="),
    );
    assert_eq!(support::output_bytes(result), b"foo");
}

#[test]
fn base32_removes_noise_and_supports_unpadded_alphabets() {
    let result = support::run(
        "encoding.base32.decode@1",
        Arguments::new(),
        support::text(" MZ XW6\n==="),
    );
    assert_eq!(support::output_bytes(result), b"foo");

    let arguments = support::argument("alphabet", ArgumentValue::Text("A-Z2-7".into()));
    let result = support::run(
        "encoding.base32.encode@1",
        arguments,
        Value::Bytes(b"foob".to_vec()),
    );
    assert_eq!(support::output_text(result), "MZXW6YQ");
}

#[test]
fn base32_replicates_the_reference_trailing_symbol_byte() {
    // CyberChef v11.3.0 decodes a lone ninth symbol into one extra byte
    // (`foobah`, not `foobar`); the byte stream is replicated exactly.
    let result = support::run(
        "encoding.base32.decode@1",
        Arguments::new(),
        support::text("MZXW6YTBM"),
    );
    assert_eq!(support::output_bytes(result), b"foobah");
}

#[test]
fn base32_rejects_inputs_the_reference_cannot_represent() {
    let strict = Arguments::from([
        ("alphabet".into(), ArgumentValue::Text("A-Z2-7=".into())),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(false)),
    ]);
    let error = support::run_with_budget(
        "encoding.base32.decode@1",
        strict,
        support::text("MZ!W6==="),
        support::budget(),
    )
    .expect_err("unfiltered invalid characters must fail");
    assert_eq!(error.code(), "encoding.base32.invalid_character");

    let unpadded = support::argument("alphabet", ArgumentValue::Text("A-Z2-7".into()));
    let error = support::run_with_budget(
        "encoding.base32.decode@1",
        unpadded,
        support::text("MY"),
        support::budget(),
    )
    .expect_err("partial groups without a padding symbol must fail");
    assert_eq!(error.code(), "encoding.base32.invalid_character");

    let invalid_alphabet = support::argument("alphabet", ArgumentValue::Text("abc".into()));
    let error = support::run_with_budget(
        "encoding.base32.encode@1",
        invalid_alphabet,
        Value::Bytes(vec![1]),
        support::budget(),
    )
    .expect_err("short alphabet must fail");
    assert_eq!(error.code(), "encoding.base32.invalid_alphabet");
}

#[test]
fn base32_preserves_the_reference_empty_input_validation_order() {
    // The reference returns an empty result before validating the alphabet.
    let invalid_alphabet = support::argument("alphabet", ArgumentValue::Text("abc".into()));
    let result = support::run(
        "encoding.base32.encode@1",
        invalid_alphabet.clone(),
        Value::Bytes(Vec::new()),
    );
    assert_eq!(support::output_text(result), "");

    let result = support::run(
        "encoding.base32.decode@1",
        invalid_alphabet,
        support::text(""),
    );
    assert_eq!(support::output_bytes(result), Vec::<u8>::new());
}
