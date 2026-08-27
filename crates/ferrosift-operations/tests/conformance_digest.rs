//! Parameters the additional digests deliberately refuse.
//!
//! The reference exposes a round count on MD2, SM3, and Whirlpool, and a
//! variant selector on Whirlpool, so that reduced-round and alternative
//! constructions can be studied. Those are not the published functions.
//! `FerroSift` implements the published ones, so it refuses the rest rather
//! than answering with a digest from a different algorithm — which would look
//! like agreement and would not be.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn run(
    operation: &str,
    arguments: Arguments,
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    support::run_with_budget(
        operation,
        arguments,
        Value::Bytes(b"FerroSift".to_vec()),
        support::budget(),
    )
}

fn integer(name: &str, value: i128) -> (String, ArgumentValue) {
    (name.into(), ArgumentValue::Integer(value))
}

#[test]
fn md2_accepts_only_its_published_round_count() {
    assert!(run("hash.md2@1", Arguments::from([integer("rounds", 18)])).is_ok());
    for rounds in [0, 1, 17, 19, 100] {
        assert!(
            run("hash.md2@1", Arguments::from([integer("rounds", rounds)])).is_err(),
            "{rounds} rounds must be refused"
        );
    }
}

#[test]
fn sm3_accepts_only_its_published_parameters() {
    let published = Arguments::from([integer("length", 256), integer("rounds", 64)]);
    assert!(run("hash.sm3@1", published).is_ok());

    let short_rounds = Arguments::from([integer("length", 256), integer("rounds", 32)]);
    assert!(run("hash.sm3@1", short_rounds).is_err());

    let other_length = Arguments::from([integer("length", 224), integer("rounds", 64)]);
    assert!(run("hash.sm3@1", other_length).is_err());
}

#[test]
fn whirlpool_accepts_only_the_final_specification() {
    let published = Arguments::from([
        ("variant".into(), ArgumentValue::Text("Whirlpool".into())),
        integer("rounds", 10),
    ]);
    assert!(run("hash.whirlpool@1", published).is_ok());

    for variant in ["Whirlpool-T", "Whirlpool-0"] {
        let arguments = Arguments::from([
            ("variant".into(), ArgumentValue::Text(variant.into())),
            integer("rounds", 10),
        ]);
        assert!(
            run("hash.whirlpool@1", arguments).is_err(),
            "{variant} must be refused"
        );
    }

    let short_rounds = Arguments::from([
        ("variant".into(), ArgumentValue::Text("Whirlpool".into())),
        integer("rounds", 5),
    ]);
    assert!(run("hash.whirlpool@1", short_rounds).is_err());
}

#[test]
fn ripemd_accepts_only_its_four_published_sizes() {
    for size in ["128", "160", "256", "320"] {
        let arguments = Arguments::from([("size".into(), ArgumentValue::Text(size.into()))]);
        assert!(run("hash.ripemd@1", arguments).is_ok(), "{size} must work");
    }
    for size in ["96", "224", "512", "abc"] {
        let arguments = Arguments::from([("size".into(), ArgumentValue::Text(size.into()))]);
        assert!(
            run("hash.ripemd@1", arguments).is_err(),
            "{size} must be refused"
        );
    }
}

fn blake3(
    size: i128,
    key: &str,
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    blake3_on(b"FerroSift", size, key)
}

/// The same, against a chosen input.
///
/// The input matters for the longest digests and only for them: the executor
/// bounds how far an operation may expand what it was given, and sixty-five
/// thousand bytes of hex from nine bytes of input is past that bound. The limit
/// is the harness's rather than the operation's, so the longest case is fed
/// enough input to stay inside it instead of being exempted from it.
fn blake3_on(
    input: &[u8],
    size: i128,
    key: &str,
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    support::run_with_budget(
        "hash.blake3@1",
        Arguments::from([
            integer("size", size),
            ("key".into(), ArgumentValue::Text(key.into())),
        ]),
        Value::Bytes(input.to_vec()),
        support::budget(),
    )
}

/// `BLAKE3`'s two refusals, which are the two its interface declares.
///
/// The digest length is free rather than chosen from a list — the output is a
/// stream, so any length is a real answer — but the reference's interface bounds
/// it at one and sixty-five thousand, and calls the upper bound arbitrary. It is
/// there to stop a recipe asking for a gigabyte of digest, and it is reproduced
/// bound and all.
#[test]
fn blake3_accepts_only_the_lengths_its_interface_offers() {
    for size in [1, 16, 32, 64] {
        assert!(blake3(size, "").is_ok(), "{size} bytes must work");
    }
    let long = [b'a'; 64];
    assert!(
        blake3_on(&long, 65_535, "").is_ok(),
        "the largest length the interface offers must work"
    );
    for size in [0, -1, 65_536, 1_000_000] {
        assert!(
            blake3_on(&long, size, "").is_err(),
            "{size} bytes must be refused"
        );
    }
}

/// A key is exactly thirty-two bytes or there is no key.
///
/// The reference refuses any other length rather than padding or hashing it into
/// one, which matters: a digest keyed with a stretched key is one nobody else
/// could reproduce. The length is measured after the same conversion the input
/// gets, so a character under two hundred and fifty-six counts as one byte.
#[test]
fn blake3_accepts_only_a_thirty_two_byte_key() {
    assert!(
        blake3(32, "").is_ok(),
        "an empty key means unkeyed, not a zero-length key"
    );
    assert!(
        blake3(32, &"k".repeat(32)).is_ok(),
        "thirty-two ASCII bytes"
    );
    assert!(
        blake3(32, &"ÿ".repeat(32)).is_ok(),
        "thirty-two characters that are each one byte"
    );

    for key in ["k", &"k".repeat(31), &"k".repeat(33)] {
        assert!(
            blake3(32, key).is_err(),
            "{} bytes must be refused",
            key.len()
        );
    }
    // Thirty-two characters, but one of them is past the byte range — so the
    // conversion falls back to UTF-8 and the key is thirty-four bytes.
    assert!(
        blake3(32, &format!("{}😀", "k".repeat(30))).is_err(),
        "thirty-two characters are not thirty-two bytes"
    );
}

/// Keyed and unkeyed `BLAKE3` are different functions.
///
/// Worth its own assertion because an empty key is the input where a port most
/// easily merges them: passing the empty slice to the keyed constructor would
/// answer a digest the reference never produces.
#[test]
fn blake3_keying_changes_the_digest() {
    let unkeyed = support::output_text(blake3(32, "").expect("unkeyed"));
    let keyed = support::output_text(blake3(32, &"k".repeat(32)).expect("keyed"));
    assert_ne!(unkeyed, keyed);
}

/// A short digest really is the prefix of a long one.
///
/// True of `BLAKE3` and false of `BLAKE2`, where the length is part of the
/// parameter block. The two live in the same module, and this is the assertion
/// that says the port did not copy one rule onto the other.
#[test]
fn blake3_shortens_by_taking_less_of_the_stream() {
    let long = support::output_text(blake3(64, "").expect("sixty-four bytes"));
    let short = support::output_text(blake3(16, "").expect("sixteen bytes"));
    assert_eq!(&long[..32], short.as_str());
}
