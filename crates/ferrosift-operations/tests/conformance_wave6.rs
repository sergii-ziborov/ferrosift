//! Conformance for AES stream modes, Key Wrap, SHA3, PBKDF2, and scrypt.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn toggle(option: &str, string: &str) -> ArgumentValue {
    ArgumentValue::Map(Arguments::from([
        ("option".into(), ArgumentValue::Text(option.into())),
        ("string".into(), ArgumentValue::Text(string.into())),
    ]))
}

fn aes_encrypt(mode: &str, plaintext: &str) -> String {
    support::output_text(support::run(
        "crypto.aes.encrypt@1",
        Arguments::from([
            (
                "key".into(),
                toggle("Hex", "00112233445566778899aabbccddeeff"),
            ),
            (
                "iv".into(),
                toggle("Hex", "0102030405060708090a0b0c0d0e0f10"),
            ),
            ("mode".into(), ArgumentValue::Text(mode.into())),
            ("input".into(), ArgumentValue::Text("Raw".into())),
            ("output".into(), ArgumentValue::Text("Hex".into())),
            ("additional_authenticated_data".into(), toggle("Hex", "")),
            (
                "include_iv_in_output".into(),
                ArgumentValue::Text("Off".into()),
            ),
        ]),
        support::text(plaintext),
    ))
}

fn aes_decrypt(mode: &str, ciphertext_hex: &str) -> Vec<u8> {
    let decrypted = support::run(
        "crypto.aes.decrypt@1",
        Arguments::from([
            (
                "key".into(),
                toggle("Hex", "00112233445566778899aabbccddeeff"),
            ),
            (
                "iv".into(),
                toggle("Hex", "0102030405060708090a0b0c0d0e0f10"),
            ),
            ("iv_length".into(), ArgumentValue::Integer(16)),
            ("mode".into(), ArgumentValue::Text(mode.into())),
            ("input".into(), ArgumentValue::Text("Hex".into())),
            ("output".into(), ArgumentValue::Text("Raw".into())),
            ("gcm_tag".into(), toggle("Hex", "")),
            ("additional_authenticated_data".into(), toggle("Hex", "")),
            ("iv_from_input".into(), ArgumentValue::Text("Off".into())),
        ]),
        support::text(ciphertext_hex),
    );
    let Value::Bytes(plain) = decrypted.value else {
        panic!("expected raw bytes");
    };
    plain
}

#[test]
fn aes_cfb_ofb_ctr_match_forge_vectors_and_round_trip() {
    let plaintext = "Attack at dawn!! more bytes here!!!";
    assert_eq!(
        aes_encrypt("CFB", plaintext),
        "fe115ddf0e1e73ca93948429b13510143ac72ada01d68942c95dbab24f3a2897f5af98"
    );
    assert_eq!(
        aes_encrypt("OFB", plaintext),
        "fe115ddf0e1e73ca93948429b13510143c0fac639fa76d4b08a3e1d36fb8fa0d85d5a0"
    );
    assert_eq!(
        aes_encrypt("CTR", plaintext),
        "fe115ddf0e1e73ca93948429b13510140919fb2b9944b8b8ffd7eaa5033c240a1bd966"
    );
    for mode in ["CFB", "OFB", "CTR"] {
        let cipher = aes_encrypt(mode, plaintext);
        assert_eq!(aes_decrypt(mode, &cipher), plaintext.as_bytes());
    }
    assert_eq!(aes_encrypt("CFB", "hi"), "d70c");
}

#[test]
fn aes_key_wrap_rfc3394_vector() {
    let wrapped = support::output_text(support::run(
        "crypto.aes_kw.wrap@1",
        Arguments::from([
            (
                "key".into(),
                toggle("Hex", "000102030405060708090a0b0c0d0e0f"),
            ),
            ("iv".into(), toggle("Hex", "a6a6a6a6a6a6a6a6")),
            ("input".into(), ArgumentValue::Text("Hex".into())),
            ("output".into(), ArgumentValue::Text("Hex".into())),
        ]),
        support::text("00112233445566778899aabbccddeeff"),
    ));
    assert_eq!(wrapped, "1fa68b0a8112b447aef34bd8fb5a7b829d3e862371d2cfe5");

    let unwrapped = support::output_text(support::run(
        "crypto.aes_kw.unwrap@1",
        Arguments::from([
            (
                "key".into(),
                toggle("Hex", "000102030405060708090a0b0c0d0e0f"),
            ),
            ("iv".into(), toggle("Hex", "a6a6a6a6a6a6a6a6")),
            ("input".into(), ArgumentValue::Text("Hex".into())),
            ("output".into(), ArgumentValue::Text("Hex".into())),
        ]),
        support::text(&wrapped),
    ));
    assert_eq!(unwrapped, "00112233445566778899aabbccddeeff");
}

#[test]
fn sha3_256_matches_reference() {
    assert_eq!(
        support::output_text(support::run(
            "hash.sha3@1",
            Arguments::from([("size".into(), ArgumentValue::Text("256".into()))]),
            Value::Bytes(b"FerroSift".to_vec()),
        )),
        "3487ff61df1500ca17ac81f566a6e682801f992a9fb043cbce1bcc21b5294c42"
    );
}

#[test]
fn pbkdf2_sha256_matches_reference() {
    assert_eq!(
        support::output_text(support::run(
            "crypto.pbkdf2@1",
            Arguments::from([
                ("passphrase".into(), toggle("UTF8", "password")),
                ("key_size".into(), ArgumentValue::Integer(128)),
                ("iterations".into(), ArgumentValue::Integer(1)),
                (
                    "hashing_function".into(),
                    ArgumentValue::Text("SHA256".into()),
                ),
                ("salt".into(), toggle("UTF8", "saltsalt")),
            ]),
            support::text(""),
        )),
        "3bec48c577653afdc2433b93a7d8ee8e"
    );
}

#[test]
fn scrypt_matches_reference_low_cost() {
    assert_eq!(
        support::output_text(support::run(
            "crypto.scrypt@1",
            Arguments::from([
                ("salt".into(), toggle("UTF8", "saltsalt")),
                ("iterations".into(), ArgumentValue::Integer(16)),
                ("memory_factor".into(), ArgumentValue::Integer(1)),
                ("parallelization_factor".into(), ArgumentValue::Integer(1)),
                ("key_length".into(), ArgumentValue::Integer(32)),
            ]),
            support::text("password"),
        )),
        "d1adc63e89b7675ba70a871bc582b8d141f21a3e612deec9e817c81043e32939"
    );
}
