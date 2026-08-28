//! Linear execution, pause, cancellation, and partial-trace contract tests.

use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

use ferrosift_core::{
    ExecutionFailure, ExecutionStatus, Executor, NeverCancelled, OperationError,
    OperationFailureCode, TraceEventKind,
};
use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, CapabilitySet, TextEncoding, TextValue, Value,
    ValueConstraint, ValueKind,
};

#[path = "support/executor.rs"]
mod executor_support;

use executor_support::{AtomicCancellation, Behavior, budget, counter, operation, recipe, step};

#[test]
fn two_steps_pass_values_and_emit_started_completed_pairs() {
    let first_calls = counter();
    let second_calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation(
            "core.append_a@1",
            Behavior::Append(vec![b'a']),
            first_calls.clone(),
        ))
        .expect("valid first operation");
    registry
        .register(operation(
            "core.append_b@1",
            Behavior::Append(vec![b'b']),
            second_calls.clone(),
        ))
        .expect("valid second operation");

    let result = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("first", "core.append_a@1"),
                step("second", "core.append_b@1"),
            ]),
            Value::Bytes(vec![]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("recipe should complete");

    assert_eq!(result.status, ExecutionStatus::Completed);
    assert_eq!(result.value, Value::Bytes(b"ab".to_vec()));
    assert_eq!(result.trace.events.len(), 4);
    assert!(matches!(
        result.trace.events[0].kind,
        TraceEventKind::StepStarted { .. }
    ));
    assert!(matches!(
        result.trace.events[3].kind,
        TraceEventKind::StepCompleted { .. }
    ));
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn defaults_are_resolved_and_disabled_takes_precedence_over_breakpoint() {
    let calls = counter();
    let mut append = operation(
        "core.append_default@1",
        Behavior::AppendArgument("suffix"),
        calls.clone(),
    );
    append.spec.arguments.push(ArgumentSpec {
        name: "suffix".into(),
        description: "Bytes to append.".into(),
        required: false,
        kind: ArgumentKind::Bytes,
        default: Some(ArgumentValue::Bytes(vec![b'!'])),
    });
    let mut registry = executor_support::registry();
    registry.register(append).expect("valid operation");

    let mut disabled = step("disabled", "core.append_default@1");
    disabled.disabled = true;
    disabled.breakpoint = true;
    let enabled = step("enabled", "core.append_default@1");
    let result = Executor::new(&registry)
        .execute(
            &recipe(vec![disabled, enabled]),
            Value::Bytes(vec![b'a']),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("default argument should be applied");

    assert_eq!(result.value, Value::Bytes(b"a!".to_vec()));
    assert!(matches!(
        result.trace.events[0].kind,
        TraceEventKind::StepSkipped { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn breakpoint_pauses_before_invocation() {
    let calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation(
            "core.identity@1",
            Behavior::Identity,
            calls.clone(),
        ))
        .expect("valid operation");
    let mut paused = step("paused", "core.identity@1");
    paused.breakpoint = true;
    let input = Value::Bytes(vec![1]);

    let result = Executor::new(&registry)
        .execute(
            &recipe(vec![paused]),
            input.clone(),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("breakpoint is a successful pause");

    assert_eq!(result.status, ExecutionStatus::Paused { step_index: 0 });
    assert_eq!(result.value, input);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        result.trace.events[0].kind,
        TraceEventKind::BreakpointReached { .. }
    ));
}

#[test]
fn cancellation_between_steps_stops_at_exact_location() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let first_calls = counter();
    let second_calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation(
            "core.signal@1",
            Behavior::Signal(cancelled.clone()),
            first_calls.clone(),
        ))
        .expect("valid first operation");
    registry
        .register(operation(
            "core.later@1",
            Behavior::Identity,
            second_calls.clone(),
        ))
        .expect("valid second operation");
    let cancellation = AtomicCancellation { cancelled };

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("signal", "core.signal@1"),
                step("later", "core.later@1"),
            ]),
            Value::Empty,
            budget(),
            &cancellation,
            CapabilitySet::new(),
        )
        .expect_err("second step must observe cancellation");

    assert!(matches!(
        error.failure,
        ExecutionFailure::Operation(OperationError::Cancelled)
    ));
    assert_eq!(error.code(), "core.operation.cancelled");
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(error.trace.events.len(), 3);
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn operation_failure_stops_later_steps_and_preserves_code() {
    let failed_calls = counter();
    let later_calls = counter();
    let failure_code = OperationFailureCode::new("test.expected").expect("valid failure code");
    let mut registry = executor_support::registry();
    registry
        .register(operation(
            "core.fail@1",
            Behavior::Fail(OperationError::Failed { code: failure_code }),
            failed_calls.clone(),
        ))
        .expect("valid failure operation");
    registry
        .register(operation(
            "core.later@1",
            Behavior::Identity,
            later_calls.clone(),
        ))
        .expect("valid later operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![
                step("fail", "core.fail@1"),
                step("later", "core.later@1"),
            ]),
            Value::Empty,
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("operation failure must stop execution");

    assert_eq!(error.code(), "test.expected");
    assert_eq!(error.location.expect("step location").index, 0);
    assert_eq!(error.trace.events.len(), 2);
    assert_eq!(failed_calls.load(Ordering::SeqCst), 1);
    assert_eq!(later_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn value_constraints_fail_at_the_responsible_step() {
    let input_calls = counter();
    let mut input_operation = operation("core.bytes@1", Behavior::Identity, input_calls.clone());
    input_operation.spec.input = ValueConstraint::Exact(ValueKind::Bytes);
    let output_calls = counter();
    let mut output_operation = operation(
        "core.bad_output@1",
        Behavior::Return(Value::Text(TextValue {
            text: "wrong".into(),
            encoding: TextEncoding::Utf8,
        })),
        output_calls.clone(),
    );
    output_operation.spec.output = ValueConstraint::Exact(ValueKind::Bytes);
    let mut registry = executor_support::registry();
    registry
        .register(input_operation)
        .expect("valid input contract");
    registry
        .register(output_operation)
        .expect("valid output contract");

    let input_error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("input", "core.bytes@1")]),
            Value::Empty,
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("wrong input representation must fail");
    assert!(matches!(
        input_error.failure,
        ExecutionFailure::InputKindMismatch
    ));
    assert_eq!(input_calls.load(Ordering::SeqCst), 0);

    let output_error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("output", "core.bad_output@1")]),
            Value::Empty,
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("wrong output representation must fail");
    assert!(matches!(
        output_error.failure,
        ExecutionFailure::OutputKindMismatch
    ));
    assert_eq!(output_calls.load(Ordering::SeqCst), 1);
}
