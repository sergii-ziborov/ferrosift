//! Malformed-input and resource-ceiling contracts.

use ferrosift_core::{Cancellation, ExecutionBudget, OperationContext};
use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

#[test]
fn malformed_hex_is_rejected_with_stable_codes() {
    let arguments = support::argument("delimiter", ArgumentValue::Text("None".into()));
    let error = support::run_with_budget(
        "encoding.hex.decode@1",
        arguments.clone(),
        support::text("abc"),
        support::budget(),
    )
    .expect_err("odd input must fail");
    assert_eq!(error.code(), "encoding.hex.odd_length");

    let error = support::run_with_budget(
        "encoding.hex.decode@1",
        arguments,
        support::text("zz"),
        support::budget(),
    )
    .expect_err("invalid digit must fail");
    assert_eq!(error.code(), "encoding.hex.invalid_digit");

    let prefixed = support::argument("delimiter", ArgumentValue::Text("0x".into()));
    let error = support::run_with_budget(
        "encoding.hex.decode@1",
        prefixed,
        support::text("0x€"),
        support::budget(),
    )
    .expect_err("non-ASCII prefixed input must fail without slicing panics");
    assert_eq!(error.code(), "encoding.hex.invalid_digit");

    let error = support::run_with_budget(
        "encoding.hex.decode@1",
        Arguments::new(),
        support::text("not-hex"),
        support::budget(),
    )
    .expect_err("automatic parsing must not silently produce empty bytes");
    assert_eq!(error.code(), "encoding.hex.odd_length");
}

#[test]
fn malformed_base64_and_alphabets_are_rejected() {
    let strict = Arguments::from([
        (
            "alphabet".into(),
            ArgumentValue::Text("A-Za-z0-9+/=".into()),
        ),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
        ("strict".into(), ArgumentValue::Boolean(true)),
    ]);
    let error = support::run_with_budget(
        "encoding.base64.decode@1",
        strict,
        support::text("A"),
        support::budget(),
    )
    .expect_err("4n+1 input must fail");
    assert_eq!(error.code(), "encoding.base64.invalid_length");

    let invalid_alphabet = support::argument("alphabet", ArgumentValue::Text("abc".into()));
    let error = support::run_with_budget(
        "encoding.base64.encode@1",
        invalid_alphabet,
        Value::Bytes(vec![1, 2, 3]),
        support::budget(),
    )
    .expect_err("short alphabet must fail");
    assert_eq!(error.code(), "encoding.base64.invalid_alphabet");

    let strict = Arguments::from([
        (
            "alphabet".into(),
            ArgumentValue::Text("A-Za-z0-9+/=".into()),
        ),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
        ("strict".into(), ArgumentValue::Boolean(true)),
    ]);
    let error = support::run_with_budget(
        "encoding.base64.decode@1",
        strict,
        support::text("Zh=="),
        support::budget(),
    )
    .expect_err("non-canonical trailing bits must fail in strict mode");
    assert_eq!(error.code(), "encoding.base64.non_canonical");

    let preserve_invalid = Arguments::from([
        (
            "alphabet".into(),
            ArgumentValue::Text("A-Za-z0-9+/=".into()),
        ),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(false)),
        ("strict".into(), ArgumentValue::Boolean(false)),
    ]);
    let error = support::run_with_budget(
        "encoding.base64.decode@1",
        preserve_invalid,
        support::text("Zm!9v"),
        support::budget(),
    )
    .expect_err("unfiltered invalid characters must fail");
    assert_eq!(error.code(), "encoding.base64.invalid_character");
}

#[test]
fn encoders_reject_outputs_above_the_operation_budget() {
    let budget = ExecutionBudget {
        max_steps: 1,
        max_input_bytes: 16,
        max_output_bytes: 3,
        max_expansion_ratio: 16,
        max_branches: 64,
        max_flow_depth: 8,
        max_operation_invocations: 1_000,
        max_total_bytes_processed: 1_048_576,
    };
    let error = support::run_with_budget(
        "encoding.hex.encode@1",
        Arguments::new(),
        Value::Bytes(vec![0xaa, 0xbb]),
        budget,
    )
    .expect_err("hex output needs five bytes");
    assert_eq!(error.code(), "core.operation.output_limit_exceeded");

    let error = support::run_with_budget(
        "encoding.base64.encode@1",
        Arguments::new(),
        Value::Bytes(vec![0xaa, 0xbb, 0xcc]),
        budget,
    )
    .expect_err("Base64 output needs four bytes");
    assert_eq!(error.code(), "core.operation.output_limit_exceeded");

    let tight = ExecutionBudget {
        max_steps: 1,
        max_input_bytes: 16,
        max_output_bytes: 2,
        max_expansion_ratio: 16,
        max_branches: 64,
        max_flow_depth: 8,
        max_operation_invocations: 1_000,
        max_total_bytes_processed: 1_048_576,
    };
    for encoder in [
        "encoding.base32.encode@1",
        "encoding.base45.encode@1",
        "encoding.base58.encode@1",
        "encoding.base85.encode@1",
        "encoding.binary.encode@1",
        "encoding.decimal.encode@1",
        "encoding.octal.encode@1",
        "encoding.url.encode@1",
    ] {
        let error = support::run_with_budget(
            encoder,
            Arguments::new(),
            Value::Bytes(vec![0xaa, 0xbb]),
            tight,
        )
        .expect_err("two input bytes cannot encode into two output bytes");
        assert_eq!(
            error.code(),
            "core.operation.output_limit_exceeded",
            "{encoder}"
        );
    }
}

#[test]
fn operation_entry_points_honor_cancellation() {
    struct Cancelled;

    impl Cancellation for Cancelled {
        fn is_cancelled(&self) -> bool {
            true
        }
    }

    let registry = support::registry();
    let identifiers: Vec<_> = registry
        .catalog()
        .map(|specification| specification.id.clone())
        .collect();
    for identifier in identifiers {
        let operation = registry.get(&identifier).expect("operation must exist");
        let mut context = OperationContext::new(
            support::budget(),
            &Cancelled,
            ferrosift_model::CapabilitySet::new(),
        );
        let error = operation
            .execute(Value::Bytes(vec![1, 2, 3]), &Arguments::new(), &mut context)
            .expect_err("cancelled operation must stop");
        assert_eq!(
            error.code(),
            "core.operation.cancelled",
            "{}",
            identifier.as_str()
        );
    }
}
