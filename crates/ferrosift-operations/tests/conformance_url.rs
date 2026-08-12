//! URL encoding conformance vectors pinned against `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn url_encode_keeps_the_reference_safe_set() {
    let input = b"hello world/path?q=1&x=%20#f-._~".to_vec();
    let result = support::run(
        "encoding.url.encode@1",
        Arguments::new(),
        Value::Bytes(input),
    );
    assert_eq!(
        support::output_text(result),
        "hello%20world/path?q=1&x=%20#f%2D%2E%5F%7E"
    );
}

#[test]
fn url_encode_can_escape_every_special_byte() {
    let arguments = support::argument("encode_all_special_chars", ArgumentValue::Boolean(true));
    let result = support::run(
        "encoding.url.encode@1",
        arguments,
        Value::Bytes(b"hello world/path?q=1".to_vec()),
    );
    assert_eq!(support::output_text(result), "hello%20world%2Fpath%3Fq%3D1");
}

#[test]
fn url_encode_passes_percent_and_escapes_high_bytes() {
    let result = support::run(
        "encoding.url.encode@1",
        Arguments::new(),
        Value::Bytes(vec![0x00, 0x25, 0x7f, 0x80, 0xff]),
    );
    assert_eq!(support::output_text(result), "%00%%7F%80%FF");
}

#[test]
fn url_decode_honors_the_plus_argument() {
    let result = support::run(
        "encoding.url.decode@1",
        Arguments::new(),
        support::text("a+b%2B%20c"),
    );
    assert_eq!(support::output_text(result), "a b+ c");

    let arguments = support::argument("treat_plus_as_space", ArgumentValue::Boolean(false));
    let result = support::run("encoding.url.decode@1", arguments, support::text("a+b%2B"));
    assert_eq!(support::output_text(result), "a+b+");
}

#[test]
fn url_decode_decodes_multi_byte_utf8_sequences() {
    let result = support::run(
        "encoding.url.decode@1",
        Arguments::new(),
        support::text("%CE%93%CE%B5%CE%B9%CE%AC"),
    );
    assert_eq!(support::output_text(result), "Γειά");

    let result = support::run(
        "encoding.url.decode@1",
        Arguments::new(),
        support::text("100%25"),
    );
    assert_eq!(support::output_text(result), "100%");
}

#[test]
fn url_decode_falls_back_to_legacy_unescape_like_the_reference() {
    // Truncated percent sequences fail `decodeURIComponent`, and the
    // reference falls back to `unescape` which decodes code units directly.
    for (input, expected) in [
        ("%E0%A4%A", "\u{e0}\u{a4}%A"),
        ("%FF", "\u{ff}"),
        ("%u0413%u0414", "\u{413}\u{414}"),
        ("100%", "100%"),
        ("%E0+%A", "\u{e0} %A"),
    ] {
        let result = support::run(
            "encoding.url.decode@1",
            Arguments::new(),
            support::text(input),
        );
        assert_eq!(support::output_text(result), expected, "{input}");
    }
}
