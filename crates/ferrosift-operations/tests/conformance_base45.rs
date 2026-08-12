//! Base45 conformance vectors pinned against `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn base45_matches_rfc_9285_vectors() {
    for (plain, encoded) in [
        (&b""[..], ""),
        (b"AB", "BB8"),
        (b"Hello!!", "%69 VD92EX0"),
        (b"base-45", "UJCLQE7W581"),
    ] {
        let encoded_result = support::run(
            "encoding.base45.encode@1",
            Arguments::new(),
            Value::Bytes(plain.to_vec()),
        );
        assert_eq!(support::output_text(encoded_result), encoded);

        let decoded_result = support::run(
            "encoding.base45.decode@1",
            Arguments::new(),
            support::text(encoded),
        );
        assert_eq!(support::output_bytes(decoded_result), plain);
    }
}

#[test]
fn base45_pads_short_groups_with_literal_zero_symbols() {
    for (input, encoded) in [
        (vec![0x00], "00"),
        (vec![0x00, 0x00], "000"),
        (vec![0xff, 0xff, 0xff], "FGWU5"),
    ] {
        let result = support::run(
            "encoding.base45.encode@1",
            Arguments::new(),
            Value::Bytes(input),
        );
        assert_eq!(support::output_text(result), encoded);
    }

    // The reference pads with the character "0" even when the alphabet
    // assigns "0" to a different position.
    let arguments = support::argument("alphabet", ArgumentValue::Text("1-9a-z0 $%*+\\-./:".into()));
    let result = support::run(
        "encoding.base45.encode@1",
        arguments,
        Value::Bytes(vec![0x01]),
    );
    assert_eq!(support::output_text(result), "20");
}

#[test]
fn base45_decodes_ietf_vectors_and_strips_noise() {
    let result = support::run(
        "encoding.base45.decode@1",
        Arguments::new(),
        support::text("QED8WEX0\n"),
    );
    assert_eq!(support::output_bytes(result), b"ietf!");
}

#[test]
fn base45_replicates_reference_short_group_masking() {
    // Two trailing symbols decode through an unconditional low-byte mask in
    // the reference, so "ZZ" (value 1610) yields 0x4a.
    let result = support::run(
        "encoding.base45.decode@1",
        Arguments::new(),
        support::text("ZZ"),
    );
    assert_eq!(support::output_bytes(result), [0x4a]);

    let result = support::run(
        "encoding.base45.decode@1",
        Arguments::new(),
        support::text("A"),
    );
    assert_eq!(support::output_bytes(result), [0x0a]);
}

#[test]
fn base45_rejects_oversized_triplets_and_foreign_characters() {
    let error = support::run_with_budget(
        "encoding.base45.decode@1",
        Arguments::new(),
        support::text(":::"),
        support::budget(),
    )
    .expect_err("triplets above 65535 must fail");
    assert_eq!(error.code(), "encoding.base45.triplet_overflow");

    let strict = Arguments::from([
        (
            "alphabet".into(),
            ArgumentValue::Text("0-9A-Z $%*+\\-./:".into()),
        ),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(false)),
    ]);
    let error = support::run_with_budget(
        "encoding.base45.decode@1",
        strict,
        support::text("ab"),
        support::budget(),
    )
    .expect_err("unfiltered invalid characters must fail");
    assert_eq!(error.code(), "encoding.base45.invalid_character");

    let invalid_alphabet = support::argument("alphabet", ArgumentValue::Text("abc".into()));
    let error = support::run_with_budget(
        "encoding.base45.encode@1",
        invalid_alphabet,
        Value::Bytes(vec![1]),
        support::budget(),
    )
    .expect_err("short alphabet must fail");
    assert_eq!(error.code(), "encoding.base45.invalid_alphabet");
}
