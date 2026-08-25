//! The unified error surface: one stable code space over every layer.

use ferrosift::prelude::*;
use ferrosift::{Error, TextEncoding, TextValue};

#[test]
fn an_unknown_operation_reports_a_stable_code_and_names_itself() {
    let error = pipeline()
        .step("encoding.does_not_exist@1", Arguments::new())
        .run_bytes(b"input")
        .expect_err("unknown operations must fail");

    assert_eq!(error.code(), "ferrosift.operation.unknown");
    assert!(
        matches!(&error, Error::UnknownOperation(name) if name == "encoding.does_not_exist@1"),
        "{error:?}"
    );
    assert!(
        alloc_display(&error).contains("encoding.does_not_exist@1"),
        "{error}"
    );
}

#[test]
fn a_malformed_operation_id_is_rejected_as_unknown() {
    let error = pipeline()
        .step("not a valid id", Arguments::new())
        .run_bytes(b"input")
        .expect_err("malformed IDs must fail");
    assert_eq!(error.code(), "ferrosift.operation.unknown");
}

#[test]
fn execution_failures_keep_the_engine_code() {
    // Base64 rejects characters outside the alphabet when filtering is off.
    let arguments = Arguments::from([
        (
            "alphabet".into(),
            ferrosift::ArgumentValue::Text("A-Za-z0-9+/=".into()),
        ),
        (
            "remove_non_alphabet".into(),
            ferrosift::ArgumentValue::Boolean(false),
        ),
        ("strict".into(), ferrosift::ArgumentValue::Boolean(false)),
    ]);
    let error = pipeline()
        .step("encoding.base64.decode@1", arguments)
        .run_bytes(b"Zm!9v")
        .expect_err("invalid characters must fail");

    assert_eq!(error.code(), "encoding.base64.invalid_character");
    assert!(matches!(error, Error::Execution(_)), "{error:?}");
}

#[test]
fn pattern_failures_keep_the_pattern_code() {
    let error = pipeline()
        .run_pattern("struct Broken {", b"data")
        .expect_err("a malformed pattern must fail");

    assert_eq!(error.code(), "pattern.parse.unexpected_token");
    assert!(matches!(error, Error::Pattern(_)), "{error:?}");
}

#[test]
fn pattern_evaluation_failures_surface_through_the_facade() {
    let error = pipeline()
        .run_pattern("u32 value @ 0;", &[1, 2])
        .expect_err("a short buffer must fail");
    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
}

#[test]
fn a_non_text_result_cannot_be_returned_as_text() {
    let error = pipeline()
        .from_base64()
        .run_text("Zm9v")
        .expect_err("byte output is not text");
    assert_eq!(error.code(), "ferrosift.output.unexpected_kind");
    assert!(matches!(error, Error::UnexpectedOutput), "{error:?}");
}

#[test]
fn every_variant_renders_a_message_and_a_code() {
    let cases = [
        pipeline()
            .step("missing@1", Arguments::new())
            .run_bytes(b"x")
            .expect_err("unknown"),
        pipeline()
            .from_base64()
            .run_text("Zm9v")
            .expect_err("unexpected output"),
        pipeline()
            .run_pattern("struct X {", b"x")
            .expect_err("pattern"),
    ];
    for error in &cases {
        assert!(!error.code().is_empty(), "{error:?}");
        assert!(!alloc_display(error).is_empty(), "{error:?}");
    }
}

#[test]
fn structured_input_is_left_for_the_engine_to_reject() {
    // Adaptation only converts between bytes and UTF-8 text; anything else is
    // passed through so the executor reports the real mismatch.
    let value = Value::Structured(ferrosift::StructuredValue::Integer(7));
    let error = pipeline()
        .to_hex()
        .run(value)
        .expect_err("structured input is not bytes");
    assert_eq!(error.code(), "core.executor.input_kind_mismatch");
}

#[test]
fn non_utf8_bytes_reach_a_text_step_the_way_the_reference_delivers_them() {
    // Not a mismatch any more. A byte-and-text step accepts either, and bytes
    // that are not valid UTF-8 are read a byte per character rather than
    // refused — which is what the reference does, and what makes `From Base64`
    // on `ff fe` an empty result there rather than an error.
    let output = pipeline()
        .from_base64()
        .run(Value::Bytes(vec![0xff, 0xfe]))
        .expect("bytes are readable as text");
    assert_eq!(output, Value::Bytes(Vec::new()));
}

#[test]
fn text_output_is_returned_as_bytes_when_bytes_are_requested() {
    let output = pipeline()
        .to_hex()
        .run_bytes(&[0x48, 0x69])
        .expect("text output converts to bytes");
    assert_eq!(output, b"48 69");
}

fn alloc_display(error: &Error) -> String {
    use core::fmt::Write as _;
    let mut text = String::new();
    let _ = write!(text, "{error}");
    text
}

#[test]
fn a_text_value_survives_when_the_first_step_wants_text() {
    let value = Value::Text(TextValue {
        text: "48 69".into(),
        encoding: TextEncoding::Utf8,
    });
    let output = pipeline().from_hex().run(value).expect("run succeeds");
    assert_eq!(output, Value::Bytes(vec![0x48, 0x69]));
}
