//! Preflight and deterministic resource ceiling tests for the executor.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ferrosift_core::{ExecutionFailure, Executor, NeverCancelled, OperationError};
use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, CapabilitySet, HostCapability, Recipe,
    RecipeMetadata, Value,
};

#[path = "support/executor.rs"]
mod executor_support;

use executor_support::{AtomicCancellation, Behavior, budget, counter, operation, recipe, step};

#[test]
fn later_unknown_operation_prevents_all_invocation() {
    let calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation("core.known@1", Behavior::Identity, calls.clone()))
        .expect("valid operation");
    let recipe = recipe(vec![
        step("known", "core.known@1"),
        step("missing", "core.missing@1"),
    ]);

    let error = Executor::new(&registry)
        .execute(
            &recipe,
            Value::Bytes(vec![1]),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("preflight must reject unknown operations");

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(error.failure, ExecutionFailure::UnknownOperation));
    assert_eq!(error.code(), "core.executor.operation_unknown");
    assert_eq!(error.location.expect("step location").index, 1);
    assert_eq!(error.trace.events.len(), 1);
    assert!(matches!(
        &error.trace.events[0].kind,
        ferrosift_core::TraceEventKind::ExecutionFailed { code }
            if code == "core.executor.operation_unknown"
    ));
}

#[test]
fn invalid_arguments_fail_preflight_without_invocation() {
    let calls = counter();
    let mut operation = operation("core.arguments@1", Behavior::Identity, calls.clone());
    operation.spec.arguments = vec![
        ArgumentSpec {
            name: "count".into(),
            description: "Required count.".into(),
            required: true,
            kind: ArgumentKind::Integer,
            default: None,
        },
        ArgumentSpec {
            name: "label".into(),
            description: "Optional label.".into(),
            required: false,
            kind: ArgumentKind::Text,
            default: Some(ArgumentValue::Text("default".into())),
        },
    ];
    let mut registry = executor_support::registry();
    registry.register(operation).expect("valid operation");

    let cases = [
        ferrosift_model::Arguments::new(),
        ferrosift_model::Arguments::from([("count".into(), ArgumentValue::Text("wrong".into()))]),
        ferrosift_model::Arguments::from([
            ("count".into(), ArgumentValue::Integer(1)),
            ("unknown".into(), ArgumentValue::Integer(2)),
        ]),
    ];
    for arguments in cases {
        let mut candidate = step("arguments", "core.arguments@1");
        candidate.arguments = arguments;
        let error = Executor::new(&registry)
            .execute(
                &recipe(vec![candidate]),
                Value::Empty,
                budget(),
                &NeverCancelled,
                CapabilitySet::new(),
            )
            .expect_err("invalid arguments must fail");

        assert!(matches!(
            error.failure,
            ExecutionFailure::Operation(OperationError::InvalidArguments)
        ));
        assert_eq!(error.code(), "core.operation.invalid_arguments");
        assert_eq!(error.location.expect("step location").index, 0);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn missing_capability_fails_closed() {
    let calls = counter();
    let mut operation = operation("core.networked@1", Behavior::Identity, calls.clone());
    operation.spec.capabilities.insert(HostCapability::Network);
    let mut registry = executor_support::registry();
    registry.register(operation).expect("valid operation");

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("network", "core.networked@1")]),
            Value::Empty,
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("missing capability must fail");

    assert!(matches!(
        error.failure,
        ExecutionFailure::CapabilityDenied {
            capability: HostCapability::Network
        }
    ));
    assert_eq!(error.code(), "core.executor.capability_denied");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn structural_and_initial_budget_failures_are_global() {
    let calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation(
            "core.identity@1",
            Behavior::Identity,
            calls.clone(),
        ))
        .expect("valid operation");
    let duplicate = step("same", "core.identity@1");
    let invalid = Recipe {
        schema_version: 1,
        steps: vec![duplicate.clone(), duplicate],
        metadata: RecipeMetadata::default(),
    };

    let duplicate_error = Executor::new(&registry)
        .execute(
            &invalid,
            Value::Empty,
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("duplicate step IDs must fail");
    assert_eq!(duplicate_error.code(), "model.recipe.duplicate_step_id");
    assert!(matches!(
        duplicate_error.failure,
        ExecutionFailure::InvalidRecipe(_)
    ));
    assert!(duplicate_error.location.is_none());

    let one_step = recipe(vec![step("identity", "core.identity@1")]);
    let mut step_budget = budget();
    step_budget.max_steps = 0;
    let step_error = Executor::new(&registry)
        .execute(
            &one_step,
            Value::Empty,
            step_budget,
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("step ceiling must fail");
    assert!(matches!(
        step_error.failure,
        ExecutionFailure::StepLimitExceeded
    ));
    assert!(step_error.location.is_none());

    let mut input_budget = budget();
    input_budget.max_input_bytes = 2;
    let input_error = Executor::new(&registry)
        .execute(
            &one_step,
            Value::Bytes(vec![1, 2, 3]),
            input_budget,
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect_err("input ceiling must fail");
    assert!(matches!(
        input_error.failure,
        ExecutionFailure::InputLimitExceeded
    ));
    assert!(input_error.location.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn disabled_unknown_operation_is_an_explicit_skip() {
    let registry = executor_support::registry();
    let mut disabled = step("disabled", "core.not_registered@1");
    disabled.disabled = true;
    let input = Value::Bytes(vec![1, 2]);

    let result = Executor::new(&registry)
        .execute(
            &recipe(vec![disabled]),
            input.clone(),
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("disabled unknown operation must be skipped");

    assert_eq!(result.value, input);
    assert_eq!(result.trace.events.len(), 1);
}

#[test]
fn initial_cancellation_is_a_global_wrapped_failure() {
    let calls = counter();
    let mut registry = executor_support::registry();
    registry
        .register(operation(
            "core.identity@1",
            Behavior::Identity,
            calls.clone(),
        ))
        .expect("valid operation");
    let cancellation = AtomicCancellation {
        cancelled: Arc::new(AtomicBool::new(true)),
    };

    let error = Executor::new(&registry)
        .execute(
            &recipe(vec![step("identity", "core.identity@1")]),
            Value::Empty,
            budget(),
            &cancellation,
            CapabilitySet::new(),
        )
        .expect_err("pre-cancelled execution must fail before a step");

    assert!(matches!(
        error.failure,
        ExecutionFailure::Operation(OperationError::Cancelled)
    ));
    assert_eq!(error.code(), "core.operation.cancelled");
    assert!(error.location.is_none());
    assert!(error.trace.events.is_empty());
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
