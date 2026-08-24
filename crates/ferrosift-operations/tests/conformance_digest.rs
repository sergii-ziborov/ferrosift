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
