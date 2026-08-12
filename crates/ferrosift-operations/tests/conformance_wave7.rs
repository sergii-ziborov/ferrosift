//! Conformance for Bzip2 and raw deflate/inflate.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn raw_deflate_inflate_round_trips() {
    let plain = b"FerroSift compression";
    let deflated = support::run(
        "compression.raw.deflate@1",
        Arguments::from([(
            "compression_type".into(),
            ArgumentValue::Text("Dynamic Huffman Coding".into()),
        )]),
        Value::Bytes(plain.to_vec()),
    );
    let Value::Bytes(compressed) = deflated.value else {
        panic!("expected compressed bytes");
    };
    assert!(!compressed.is_empty());

    let inflated = support::run(
        "compression.raw.inflate@1",
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
        Value::Bytes(compressed),
    );
    let Value::Bytes(out) = inflated.value else {
        panic!("expected inflated bytes");
    };
    assert_eq!(out, plain);
}

#[test]
fn raw_inflate_matches_cyberchef_payload() {
    // CyberChef Raw Deflate of "FerroSift compression" (Dynamic Huffman).
    let cc_hex = "0540c1090020105aa5696e814608031f6568eb3f8e82adc9fdc7d2794642dd06";
    let mut digits = Vec::new();
    for ch in cc_hex.chars() {
        if ch.is_ascii_hexdigit() {
            digits.push(ch);
        }
    }
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for chunk in digits.chunks(2) {
        let text: String = chunk.iter().collect();
        bytes.push(u8::from_str_radix(&text, 16).unwrap());
    }

    let inflated = support::run(
        "compression.raw.inflate@1",
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
        Value::Bytes(bytes),
    );
    let Value::Bytes(out) = inflated.value else {
        panic!("expected inflated bytes");
    };
    assert_eq!(out, b"FerroSift compression");
}

#[test]
fn bzip2_round_trips_and_inflates_cyberchef_payload() {
    let plain = b"FerroSift compression";
    let compressed = support::run(
        "compression.bzip2.compress@1",
        Arguments::from([
            ("block_size".into(), ArgumentValue::Integer(9)),
            ("work_factor".into(), ArgumentValue::Integer(30)),
        ]),
        Value::Bytes(plain.to_vec()),
    );
    let Value::Bytes(bz) = compressed.value else {
        panic!("expected bzip2 bytes");
    };
    assert!(bz.starts_with(b"BZh"));

    let decompressed = support::run(
        "compression.bzip2.decompress@1",
        Arguments::from([("low_memory".into(), ArgumentValue::Boolean(false))]),
        Value::Bytes(bz),
    );
    let Value::Bytes(out) = decompressed.value else {
        panic!("expected plain bytes");
    };
    assert_eq!(out, plain);

    // CyberChef Bzip2 Compress of the same input (block 9, work 30).
    let cc_hex = "425a683931415926535911be4bc300000097804000010008000b23dc0020003100000843d469e93ca337a0ae9f201da61561fc5dc914e1424046f92f0c";
    let mut digits = Vec::new();
    for ch in cc_hex.chars() {
        if ch.is_ascii_hexdigit() {
            digits.push(ch);
        }
    }
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for chunk in digits.chunks(2) {
        let text: String = chunk.iter().collect();
        bytes.push(u8::from_str_radix(&text, 16).unwrap());
    }
    let from_cc = support::run(
        "compression.bzip2.decompress@1",
        Arguments::from([("low_memory".into(), ArgumentValue::Boolean(false))]),
        Value::Bytes(bytes),
    );
    let Value::Bytes(from_cc) = from_cc.value else {
        panic!("expected plain bytes from CC payload");
    };
    assert_eq!(from_cc, plain);
}
