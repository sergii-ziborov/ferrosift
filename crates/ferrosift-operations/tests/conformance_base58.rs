//! Base58 conformance vectors pinned against `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

const RIPPLE: &str = "rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";

#[test]
fn base58_round_trips_bitcoin_vectors() {
    for (plain, encoded) in [
        (&b""[..], ""),
        (b"hello world", "StV1DL6CwTryKyV"),
        (&[0x00, 0x00, 0x01][..], "112"),
        (&[0x00, 0xff][..], "15Q"),
    ] {
        let encoded_result = support::run(
            "encoding.base58.encode@1",
            Arguments::new(),
            Value::Bytes(plain.to_vec()),
        );
        assert_eq!(support::output_text(encoded_result), encoded);

        let decoded_result = support::run(
            "encoding.base58.decode@1",
            Arguments::new(),
            support::text(encoded),
        );
        assert_eq!(support::output_bytes(decoded_result), plain);
    }
}

#[test]
fn base58_supports_the_ripple_alphabet() {
    let arguments = support::argument("alphabet", ArgumentValue::Text(RIPPLE.into()));
    let result = support::run(
        "encoding.base58.encode@1",
        arguments.clone(),
        Value::Bytes(b"hello world".to_vec()),
    );
    assert_eq!(support::output_text(result), "StVrDLaUATiyKyV");

    let result = support::run(
        "encoding.base58.decode@1",
        arguments,
        support::text("StVrDLaUATiyKyV"),
    );
    assert_eq!(support::output_bytes(result), b"hello world");
}

#[test]
fn base58_counts_leading_zeros_before_noise_removal() {
    // The reference counts leading zero symbols on the raw input, so noise
    // ahead of them silently drops the zero bytes; replicated byte-for-byte.
    let result = support::run(
        "encoding.base58.decode@1",
        Arguments::new(),
        support::text(" 112"),
    );
    assert_eq!(support::output_bytes(result), [0x01]);

    let result = support::run(
        "encoding.base58.decode@1",
        Arguments::new(),
        support::text("1"),
    );
    assert_eq!(support::output_bytes(result), [0x00]);
}

#[test]
fn base58_rejects_foreign_characters_and_bad_alphabets() {
    let strict = Arguments::from([
        (
            "alphabet".into(),
            ArgumentValue::Text(
                "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".into(),
            ),
        ),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(false)),
    ]);
    let error = support::run_with_budget(
        "encoding.base58.decode@1",
        strict,
        support::text("z!"),
        support::budget(),
    )
    .expect_err("unfiltered invalid characters must fail");
    assert_eq!(error.code(), "encoding.base58.invalid_character");

    // The reference validates the alphabet before its empty-input return,
    // so even an empty encode fails on a bad alphabet.
    let invalid_alphabet = support::argument("alphabet", ArgumentValue::Text("abc".into()));
    let error = support::run_with_budget(
        "encoding.base58.encode@1",
        invalid_alphabet,
        Value::Bytes(Vec::new()),
        support::budget(),
    )
    .expect_err("bad alphabet must fail before the empty-input return");
    assert_eq!(error.code(), "encoding.base58.invalid_alphabet");
}
