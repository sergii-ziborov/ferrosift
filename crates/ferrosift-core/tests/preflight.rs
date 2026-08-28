//! Cross-step compatibility and preflight ordering regression tests.

use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ferrosift_core::{ExecutionFailure, Executor, NeverCancelled};
use ferrosift_model::{CapabilitySet, Value, ValueConstraint, ValueKind};

#[path = "support/executor.rs"]
mod executor_support;

use executor_support::{AtomicCancellation, Behavior, budget, counter, operation, recipe, step};

#[test]
fn validation_performs_complete_preflight_without_invoking_operations() {
    let calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation("core.known@1", Behavior::Identity, calls.clone()))
        .expect("valid operation");

    Executor::new(&registry)
        .validate(
            &recipe(vec![step("known", "core.known@1")]),
            &Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            &CapabilitySet::new(),
        )
        .expect("valid recipe must pass preflight");

    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn deterministically_incompatible_later_input_fails_before_side_effects() {
    let first_calls = counter();
    let second_calls = counter();
    // Files against text, rather than bytes against text. The reference
    // converts bytes to a string on every recipe that ends in a text
    // operation, so rejecting *that* pair would reject recipes it runs. A file
    // list has no counterpart among its dish types and no byte form here, so
    // it is a flow that genuinely cannot happen -- which is what preflight is
    // for.
    let mut first = operation("core.files@1", Behavior::Identity, first_calls.clone());
    first.spec.output = ValueConstraint::Exact(ValueKind::Files);
    let mut second = operation("core.text@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Text);
    let mut registry = executor_support::registry();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("files", "core.files@1"),
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
    first.spec.output = ValueConstraint::OneOf(BTreeSet::from([ValueKind::Files, ValueKind::Text]));
    let mut second = operation("core.text@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Text);
    let mut registry = executor_support::registry();
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

/// A step that accepts and returns anything is transparent to the type check.
///
/// This used to be a preflight failure, and the failure was wrong. An `Any`
/// output was carried forward as "the next step might receive any kind", and
/// the check then demanded that *every* kind flow into that step — including
/// `Empty`, `Boolean` and `Files`, which have no byte form and so convert to
/// nothing. Nothing could ever follow such a step, which made `Identity`,
/// `Comment` and `Label` unusable in front of a typed operation: a legal recipe
/// refused before the first invocation by a question that could not be answered
/// yes.
///
/// The pair is skipped instead. What the check gives up is an assumption it was
/// never entitled to make; see the next test for what still catches it.
#[test]
fn an_unconstrained_step_is_transparent_to_the_type_check() {
    let first_calls = counter();
    let second_calls = counter();
    let first = operation("core.any@1", Behavior::Identity, first_calls.clone());
    let mut second = operation("core.bytes@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Bytes);
    let mut registry = executor_support::registry();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let result = Executor::new(&registry)
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
        .expect("a marker in front of a typed step must not refuse the recipe");

    assert_eq!(result.value, Value::Bytes(vec![1]));
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

/// A transparent step that does change the kind fails at the step that got it.
///
/// The cost of the rule above, paid where it belongs. Preflight lets the recipe
/// start because it cannot know; the per-step check refuses the value the
/// moment it arrives, naming the step that could not take it — which is more
/// than the old rule managed, because the old rule never let any recipe with a
/// marker in it start at all.
#[test]
fn a_transparent_step_that_changes_the_kind_fails_at_the_step() {
    let first_calls = counter();
    let second_calls = counter();
    let first = operation(
        "core.any@1",
        Behavior::Return(Value::Files(Vec::new())),
        first_calls.clone(),
    );
    let mut second = operation("core.text@1", Behavior::Identity, second_calls.clone());
    second.spec.input = ValueConstraint::Exact(ValueKind::Text);
    let mut registry = executor_support::registry();
    registry.register(first).expect("valid first operation");
    registry.register(second).expect("valid second operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("any", "core.any@1"), step("text", "core.text@1")]),
            Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("a file list cannot reach a step that wants text");

    assert!(matches!(error.failure, ExecutionFailure::InputKindMismatch));
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn unconstrained_output_accepts_a_semantically_complete_one_of() {
    let first_calls = counter();
    let second_calls = counter();
    let first = operation("core.any@1", Behavior::Identity, first_calls.clone());
    let mut second = operation("core.all_kinds@1", Behavior::Identity, second_calls.clone());
    // Built from `ValueKind::ALL` rather than typed out. The list here used to
    // name seven kinds, which was every kind when it was written and stopped
    // being so when three were added -- and nothing would have said so.
    second.spec.input = ValueConstraint::OneOf(ValueKind::ALL.into_iter().collect::<BTreeSet<_>>());
    let mut registry = executor_support::registry();
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
    let mut registry = executor_support::registry();
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
