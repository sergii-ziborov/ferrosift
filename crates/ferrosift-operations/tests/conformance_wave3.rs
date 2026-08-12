//! Conformance for hash, compression, HTML, ROT13, and charcode ops.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn hashes_match_reference_vectors() {
    let hello = Value::Bytes(b"hello".to_vec());
    assert_eq!(
        support::output_text(support::run("hash.md5@1", Arguments::new(), hello.clone())),
        "5d41402abc4b2a76b9719d911017c592"
    );
    assert_eq!(
        support::output_text(support::run(
            "hash.sha1@1",
            support::argument("rounds", ArgumentValue::Integer(80)),
            hello.clone(),
        )),
        "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
    );
    assert_eq!(
        support::output_text(support::run(
            "hash.sha2@1",
            Arguments::from([
                ("size".into(), ArgumentValue::Text("256".into())),
                ("rounds_256".into(), ArgumentValue::Integer(64)),
                ("rounds_512".into(), ArgumentValue::Integer(160)),
            ]),
            hello.clone(),
        )),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    assert_eq!(
        support::output_text(support::run(
            "hash.hmac@1",
            Arguments::from([
                (
                    "key".into(),
                    ArgumentValue::Map(Arguments::from([
                        ("option".into(), ArgumentValue::Text("UTF8".into())),
                        ("string".into(), ArgumentValue::Text("key".into())),
                    ])),
                ),
                (
                    "hashing_function".into(),
                    ArgumentValue::Text("SHA256".into()),
                ),
            ]),
            hello,
        )),
        "9307b3b915efb5171ff14d8cb55fbcc798c6c0ef1456d66ded1a6aa723a58b7b"
    );
}

#[test]
fn zlib_and_gzip_round_trip() {
    let payload = Value::Bytes(b"hello ferro".to_vec());
    let deflated = support::output_bytes(support::run(
        "compression.zlib.deflate@1",
        support::argument(
            "compression_type",
            ArgumentValue::Text("Dynamic Huffman Coding".into()),
        ),
        payload.clone(),
    ));
    let inflated = support::output_bytes(support::run(
        "compression.zlib.inflate@1",
        Arguments::from([
            ("start_index".into(), ArgumentValue::Integer(0)),
            (
                "initial_output_buffer_size".into(),
                ArgumentValue::Integer(0),
            ),
            (
                "buffer_expansion_type".into(),
                ArgumentValue::Text("Adaptive".into()),
            ),
            (
                "resize_buffer_after_decompression".into(),
                ArgumentValue::Boolean(false),
            ),
            ("verify_result".into(), ArgumentValue::Boolean(false)),
        ]),
        Value::Bytes(deflated),
    ));
    assert_eq!(inflated, b"hello ferro");

    let gzipped = support::output_bytes(support::run(
        "compression.gzip@1",
        Arguments::from([
            (
                "compression_type".into(),
                ArgumentValue::Text("Dynamic Huffman Coding".into()),
            ),
            ("filename".into(), ArgumentValue::Text(String::new())),
            ("comment".into(), ArgumentValue::Text(String::new())),
            (
                "include_file_checksum".into(),
                ArgumentValue::Boolean(false),
            ),
        ]),
        payload,
    ));
    assert_eq!(
        support::output_bytes(support::run(
            "compression.gunzip@1",
            Arguments::new(),
            Value::Bytes(gzipped),
        )),
        b"hello ferro"
    );
}

#[test]
fn html_rot13_and_charcode_match_reference_cases() {
    assert_eq!(
        support::output_text(support::run(
            "encoding.html.encode@1",
            Arguments::from([
                (
                    "convert_all_characters".into(),
                    ArgumentValue::Boolean(false),
                ),
                (
                    "convert_to".into(),
                    ArgumentValue::Text("Named entities".into()),
                ),
            ]),
            support::text("a & b <c>"),
        )),
        "a &amp; b &lt;c&gt;"
    );
    assert_eq!(
        support::output_text(support::run(
            "encoding.html.decode@1",
            Arguments::new(),
            support::text("a &amp; b &lt;c&gt;"),
        )),
        "a & b <c>"
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.rot13@1",
            Arguments::from([
                (
                    "rotate_lower_case_chars".into(),
                    ArgumentValue::Boolean(true),
                ),
                (
                    "rotate_upper_case_chars".into(),
                    ArgumentValue::Boolean(true),
                ),
                ("rotate_numbers".into(), ArgumentValue::Boolean(false)),
                ("amount".into(), ArgumentValue::Integer(13)),
            ]),
            Value::Bytes(b"Hello, World!".to_vec()),
        )),
        b"Uryyb, Jbeyq!"
    );
    assert_eq!(
        support::output_text(support::run(
            "encoding.charcode.encode@1",
            Arguments::from([
                ("delimiter".into(), ArgumentValue::Text("Space".into())),
                ("base".into(), ArgumentValue::Integer(16)),
            ]),
            support::text("Hi"),
        )),
        "48 69"
    );
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.charcode.decode@1",
            Arguments::from([
                ("delimiter".into(), ArgumentValue::Text("Space".into())),
                ("base".into(), ArgumentValue::Integer(16)),
            ]),
            support::text("48 69"),
        )),
        b"Hi"
    );
}
