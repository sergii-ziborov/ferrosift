//! Cross-step compatibility and preflight ordering regression tests.

use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ferrosift_core::{ExecutionFailure, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{CapabilitySet, Value, ValueConstraint, ValueKind};

#[path = "support/executor.rs"]
mod executor_support;

use executor_support::{AtomicCancellation, Behavior, budget, counter, operation, recipe, step};

#[test]
fn deterministically_incompatible_later_input_fails_before_side_effects() {
    let first_calls = counter();
    let second_calls = counter();
    let mut first = operation("core.bytes@1", Behavior::Identity, first_calls.clone());
    first.spec.output = ValueConstraint::Exact(ValueKind::Bytes);
    let mut second = operation("core.text@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Text);
    let mut registry = OperationRegistry::new();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("bytes", "core.bytes@1"),
                step("text", "core.text@1"),
            ]),
            Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("incompatible declared chain must fail preflight");

    assert!(matches!(error.failure, ExecutionFailure::InputKindMismatch));
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(error.trace.events.len(), 1);
    assert_eq!(first_calls.load(Ordering::SeqCst), 0);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn partially_overlapping_output_contract_fails_before_side_effects() {
    let first_calls = counter();
    let second_calls = counter();
    let mut first = operation("core.either@1", Behavior::Identity, first_calls.clone());
    first.spec.output = ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text]));
    let mut second = operation("core.text@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Text);
    let mut registry = OperationRegistry::new();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("either", "core.either@1"),
                step("text", "core.text@1"),
            ]),
            Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("every declared output kind must satisfy the next input");

    assert!(matches!(error.failure, ExecutionFailure::InputKindMismatch));
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(error.trace.events.len(), 1);
    assert_eq!(first_calls.load(Ordering::SeqCst), 0);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn unconstrained_output_requires_an_unconstrained_downstream_input() {
    let first_calls = counter();
    let second_calls = counter();
    let first = operation("core.any@1", Behavior::Identity, first_calls.clone());
    let mut second = operation("core.bytes@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Bytes);
    let mut registry = OperationRegistry::new();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("any", "core.any@1"),
                step("bytes", "core.bytes@1"),
            ]),
            Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("an unconstrained output cannot guarantee bytes");

    assert!(matches!(error.failure, ExecutionFailure::InputKindMismatch));
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(first_calls.load(Ordering::SeqCst), 0);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn unconstrained_output_accepts_a_semantically_complete_one_of() {
    let first_calls = counter();
    let second_calls = counter();
    let first = operation("core.any@1", Behavior::Identity, first_calls.clone());
    let mut second = operation("core.all_kinds@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::OneOf(BTreeSet::from([
        ValueKind::Empty,
        ValueKind::Bytes,
        ValueKind::Text,
        ValueKind::Boolean,
        ValueKind::Integer,
        ValueKind::Structured,
        ValueKind::Files,
    ]));
    let mut registry = OperationRegistry::new();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let result = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("any", "core.any@1"),
                step("all", "core.all_kinds@1"),
            ]),
            Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("all-kind constraint is semantically unconstrained");

    assert_eq!(result.value, Value::Bytes(vec![1]));
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn complete_preflight_reports_invalid_steps_before_cancellation() {
    let calls = counter();
    let mut registry = OperationRegistry::new();
    registry
        .register(operation("core.known@1", Behavior::Identity, calls.clone()))
        .expect("valid operation");
    let cancellation = AtomicCancellation {
        cancelled: Arc::new(AtomicBool::new(true)),
    };

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("known", "core.known@1"),
                step("missing", "core.missing@1"),
            ]),
            Value::Empty,
            budget(),
            &cancellation,
            CapabilitySet::new(),
        )
        .expect_err("complete preflight must identify the invalid later step");

    assert!(matches!(error.failure, ExecutionFailure::UnknownOperation));
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
