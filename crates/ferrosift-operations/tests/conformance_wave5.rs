//! Conformance for AES, RC4, and XOR brute force.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn toggle(option: &str, string: &str) -> ArgumentValue {
    ArgumentValue::Map(Arguments::from([
        ("option".into(), ArgumentValue::Text(option.into())),
        ("string".into(), ArgumentValue::Text(string.into())),
    ]))
}

#[test]
fn aes_cbc_matches_reference_and_round_trips() {
    let encrypted = support::output_text(support::run(
        "crypto.aes.encrypt@1",
        Arguments::from([
            (
                "key".into(),
                toggle("Hex", "00112233445566778899aabbccddeeff"),
            ),
            (
                "iv".into(),
                toggle("Hex", "00000000000000000000000000000000"),
            ),
            ("mode".into(), ArgumentValue::Text("CBC".into())),
            ("input".into(), ArgumentValue::Text("Raw".into())),
            ("output".into(), ArgumentValue::Text("Hex".into())),
            ("additional_authenticated_data".into(), toggle("Hex", "")),
            (
                "include_iv_in_output".into(),
                ArgumentValue::Text("Off".into()),
            ),
        ]),
        support::text("Attack at dawn!!"),
    ));
    assert_eq!(
        encrypted,
        "43fd4535ebfdd0fad8aea4675f7846cd85d84605fb473bc511dc9ebfed455e93"
    );

    let decrypted = support::run(
        "crypto.aes.decrypt@1",
        Arguments::from([
            (
                "key".into(),
                toggle("Hex", "00112233445566778899aabbccddeeff"),
            ),
            (
                "iv".into(),
                toggle("Hex", "00000000000000000000000000000000"),
            ),
            ("iv_length".into(), ArgumentValue::Integer(16)),
            ("mode".into(), ArgumentValue::Text("CBC".into())),
            ("input".into(), ArgumentValue::Text("Hex".into())),
            ("output".into(), ArgumentValue::Text("Raw".into())),
            ("gcm_tag".into(), toggle("Hex", "")),
            ("additional_authenticated_data".into(), toggle("Hex", "")),
            ("iv_from_input".into(), ArgumentValue::Text("Off".into())),
        ]),
        support::text(&encrypted),
    );
    let Value::Bytes(plain) = decrypted.value else {
        panic!("expected raw bytes");
    };
    assert_eq!(plain, b"Attack at dawn!!");
}

#[test]
fn rc4_matches_reference_hex_vector() {
    assert_eq!(
        support::output_text(support::run(
            "crypto.rc4@1",
            Arguments::from([
                ("passphrase".into(), toggle("UTF8", "secret")),
                ("input_format".into(), ArgumentValue::Text("UTF8".into())),
                ("output_format".into(), ArgumentValue::Text("Hex".into())),
            ]),
            support::text("Hello"),
        )),
        "a553be70ed"
    );
}

#[test]
fn xor_brute_force_lists_key_one_sample() {
    let output = support::output_text(support::run(
        "logic.xor_brute@1",
        Arguments::from([
            ("key_length".into(), ArgumentValue::Integer(1)),
            ("sample_length".into(), ArgumentValue::Integer(5)),
            ("sample_offset".into(), ArgumentValue::Integer(0)),
            ("scheme".into(), ArgumentValue::Text("Standard".into())),
            ("null_preserving".into(), ArgumentValue::Boolean(false)),
            ("print_key".into(), ArgumentValue::Boolean(true)),
            ("output_as_hex".into(), ArgumentValue::Boolean(true)),
            ("crib".into(), ArgumentValue::Text(String::new())),
        ]),
        Value::Bytes(vec![0x1f, 0x00, 0x1a, 0x1b, 0x00]),
    ));
    assert!(output.starts_with("Key = 01: 1e 01 1b 1a 01\nKey = 02:"));
    assert!(output.contains("Key = ff:"));
}
