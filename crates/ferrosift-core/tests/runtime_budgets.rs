//! Runtime output and expansion ceilings for the bounded executor.

use std::sync::atomic::Ordering;

use ferrosift_core::{ExecutionFailure, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{CapabilitySet, Value};

#[path = "support/executor.rs"]
mod executor_support;

use executor_support::{Behavior, budget, counter, operation, recipe, step};

#[test]
fn output_ceiling_fails_after_invocation_with_partial_trace() {
    let calls = counter();
    let mut registry = OperationRegistry::new();
    registry
        .register(operation(
            "core.expand@1",
            Behavior::Append(vec![2, 3, 4]),
            calls.clone(),
        ))
        .expect("valid operation");
    let mut limit = budget();
    limit.max_output_bytes = 3;

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("expand", "core.expand@1")]),
            Value::Bytes(vec![1]),
            limit,
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("output ceiling must fail");

    assert!(matches!(
        error.failure,
        ExecutionFailure::OutputLimitExceeded
    ));
    assert_eq!(error.code(), "core.executor.output_limit_exceeded");
    assert_eq!(error.location.expect("step location").index, 0);
    assert_eq!(error.trace.events.len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn per_step_expansion_uses_a_nonzero_denominator() {
    let calls = counter();
    let mut registry = OperationRegistry::new();
    registry
        .register(operation(
            "core.expand@1",
            Behavior::Return(Value::Bytes(vec![1, 2, 3])),
            calls.clone(),
        ))
        .expect("valid operation");
    let mut limit = budget();
    limit.max_expansion_ratio = 2;

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("expand", "core.expand@1")]),
            Value::Bytes(Vec::new()),
            limit,
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("zero-byte input must still have a bounded ratio");

    assert!(matches!(
        error.failure,
        ExecutionFailure::ExpansionRatioExceeded
    ));
    assert_eq!(error.code(), "core.executor.expansion_ratio_exceeded");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn total_expansion_catches_safe_individual_steps() {
    let first_calls = counter();
    let second_calls = counter();
    let mut registry = OperationRegistry::new();
    registry
        .register(operation(
            "core.double_one@1",
            Behavior::Append(vec![1]),
            first_calls.clone(),
        ))
        .expect("valid first operation");
    registry
        .register(operation(
            "core.double_two@1",
            Behavior::Append(vec![2, 3]),
            second_calls.clone(),
        ))
        .expect("valid second operation");
    let mut limit = budget();
    limit.max_expansion_ratio = 2;

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("first", "core.double_one@1"),
                step("second", "core.double_two@1"),
            ]),
            Value::Bytes(vec![0]),
            limit,
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("total ratio must catch chained expansion");

    assert!(matches!(
        error.failure,
        ExecutionFailure::ExpansionRatioExceeded
    ));
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(error.trace.events.len(), 4);
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}
