//! The two readings of a toggleString field, and the one place they cannot be.
//!
//! `Utils.convertToByteArray` and `Utils.convertToByteString` are two functions
//! in the reference, and which one an operation calls decides what its key *is*.
//! The corpus pins twenty-five fields through both, which proves they agree
//! where they agree. This file holds the part a corpus cannot: that they
//! deliberately *dis*agree, and the single input where the second reading has no
//! byte-oriented answer at all.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

/// A toggleString argument, as the model carries one.
fn toggle(name: &str, option: &str, string: &str) -> (String, ArgumentValue) {
    (
        name.into(),
        ArgumentValue::Map(Arguments::from([
            ("option".into(), ArgumentValue::Text(option.into())),
            ("string".into(), ArgumentValue::Text(string.into())),
        ])),
    )
}

/// XOR against zeros, so the output *is* the key bytes repeated.
fn xor_key_bytes(option: &str, string: &str) -> Vec<u8> {
    let result = support::run(
        "logic.xor@1",
        Arguments::from([
            toggle("key", option, string),
            ("scheme".into(), ArgumentValue::Text("Standard".into())),
            ("null_preserving".into(), ArgumentValue::Boolean(false)),
        ]),
        Value::Bytes(vec![0; 16]),
    );
    match result.value {
        Value::Bytes(bytes) => bytes,
        other => panic!("XOR must produce bytes, got {other:?}"),
    }
}

/// The array reading, which is XOR's.
///
/// Every row is an input where the obvious strict port refuses or splits
/// differently, and the reference does neither.
#[test]
fn the_array_reading_is_the_permissive_one() {
    // Hex splits on anything that is not a digit and reads each run in pairs,
    // so an odd digit is a byte with a zero high nibble.
    assert_eq!(&xor_key_bytes("Hex", "abc")[..3], &[0xab, 0x0c, 0xab]);
    // `0x` is consumed whole rather than as a digit and a letter.
    assert_eq!(&xor_key_bytes("Hex", "0x41 0x42")[..2], b"AB");
    // Every other separator is just a gap between runs.
    assert_eq!(&xor_key_bytes("Hex", "de:ad")[..2], &[0xde, 0xad]);
    // And a field with no digits at all is empty, not an error.
    assert_eq!(xor_key_bytes("Hex", "zz"), vec![0; 16]);

    // Base64 removes what is not in the alphabet instead of refusing it.
    assert_eq!(&xor_key_bytes("Base64", "!QUJD!")[..3], b"ABC");
    // An unpadded tail yields only the bytes it can fill.
    assert_eq!(&xor_key_bytes("Base64", "QU")[..1], b"A");

    // Binary removes the whitespace and then chunks what is left, running
    // across where the gaps were rather than restarting at them.
    assert_eq!(&xor_key_bytes("Binary", "0100 000101000010")[..2], b"AB");

    // Decimal's separator is any run of characters that are not digits.
    assert_eq!(&xor_key_bytes("Decimal", "65;66;67")[..3], b"ABC");
}

/// Latin1 is where the two readings part, and an unknown name reads as Latin1.
#[test]
fn the_array_reading_falls_back_to_utf_eight() {
    // Every character fits in a byte, so the code units are the bytes.
    assert_eq!(&xor_key_bytes("Latin1", "aÿz")[..3], &[0x61, 0xff, 0x7a]);
    // One does not, so the whole string is UTF-8 encoded instead — six bytes
    // from two characters, not two.
    assert_eq!(
        &xor_key_bytes("Latin1", "日本")[..6],
        "日本".as_bytes(),
        "past the byte range the reference encodes rather than masks"
    );
    // The reference's `switch` has no error branch: an unrecognised option name
    // falls through to the Latin1 case rather than being refused.
    assert_eq!(
        xor_key_bytes("Nonsense", "abc"),
        xor_key_bytes("Latin1", "abc")
    );
}

/// HMAC's key, which takes the *other* reading.
fn hmac_digest(option: &str, string: &str) -> String {
    let result = support::run(
        "hash.hmac@1",
        Arguments::from([
            toggle("key", option, string),
            (
                "hashing_function".into(),
                ArgumentValue::Text("SHA256".into()),
            ),
        ]),
        support::text("message"),
    );
    support::output_text(result)
}

/// The same field, read the other way, is a different key.
///
/// This is the assertion that stops the two readings being merged: if HMAC ever
/// starts agreeing with XOR here, one of them has been given the other's
/// function.
#[test]
fn the_string_reading_masks_where_the_array_reading_encodes() {
    // `日本` is `e5 2c` to HMAC and six UTF-8 bytes to XOR. Pinned as the digest
    // of the masked bytes, which is what the reading says the key is.
    let masked = hmac_digest("Latin1", "\u{e5}\u{2c}");
    assert_eq!(
        hmac_digest("Latin1", "日本"),
        masked,
        "the string reading keeps the low byte of each code unit"
    );
    assert_ne!(
        hmac_digest("Latin1", "日本"),
        hmac_digest("UTF8", "日本"),
        "and is not the UTF-8 encoding the array reading would give"
    );
}

/// Where the two readings agree, they agree exactly.
///
/// Worth its own test because the previous one could be satisfied by two
/// readings that differ everywhere, which is not what the reference does.
#[test]
fn the_two_readings_agree_on_every_decoded_format() {
    for (option, string, bytes) in [
        ("Hex", "41424344", &[0x41, 0x42, 0x43, 0x44][..]),
        ("Hex", "abc", &[0xab, 0x0c]),
        ("Base64", "!QUJD!", b"ABC"),
        ("Binary", "0100 000101000010", b"AB"),
        ("Decimal", "65;66;67", b"ABC"),
        ("UTF8", "日本", "日本".as_bytes()),
    ] {
        // The array reading, seen directly as the bytes XOR repeats.
        assert_eq!(
            &xor_key_bytes(option, string)[..bytes.len()],
            bytes,
            "{option} {string} through the array reading"
        );
        // The string reading, seen through a Latin1 field holding those same
        // bytes as characters — which the masking leaves exactly as they are.
        let latin1: String = bytes.iter().map(|byte| char::from(*byte)).collect();
        assert_eq!(
            hmac_digest(option, string),
            hmac_digest("Latin1", &latin1),
            "{option} {string} through the string reading"
        );
    }
}
