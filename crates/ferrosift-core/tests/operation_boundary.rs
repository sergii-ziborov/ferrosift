//! Contract tests for portable operation execution.

use std::collections::BTreeSet;

use ferrosift_core::{
    Cancellation, ExecutionBudget, NeverCancelled, Operation, OperationContext, OperationError,
    OperationFailureCode,
};
use ferrosift_model::{
    Arguments, CapabilitySet, ClassificationSet, HostCapability, OperationId, OperationSpec,
    OutputBehavior, StreamingSupport, Target, TargetSet, Value, ValueConstraint,
};

struct IdentityOperation {
    spec: OperationSpec,
}

impl Operation for IdentityOperation {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        Ok(input)
    }
}

struct Cancelled;

impl Cancellation for Cancelled {
    fn is_cancelled(&self) -> bool {
        true
    }
}

fn operation_spec() -> OperationSpec {
    let targets = TargetSet::from([Target::Native]);

    OperationSpec {
        id: OperationId::new("core.identity@1").expect("valid operation id"),
        display_name: "Identity".into(),
        category: "Core".into(),
        description: "Returns the input without conversion.".into(),
        aliases: Vec::new(),
        input: ValueConstraint::Any,
        output: ValueConstraint::Any,
        arguments: Vec::new(),
        targets,
        capabilities: CapabilitySet::new(),
        classifications: ClassificationSet::new(),
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        output_behavior: OutputBehavior::default(),
        inverse: None,
    }
}

fn budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 16,
        max_input_bytes: 1_024,
        max_output_bytes: 2_048,
        max_expansion_ratio: 8,
        max_branches: 1_024,
        max_flow_depth: 16,
        max_operation_invocations: 10_000,
        max_total_bytes_processed: 1_048_576,
        max_transient_bytes: 256 * 1024 * 1024,
        max_work_units: 1 << 26,
    }
}

#[test]
fn operation_uses_only_portable_values_and_explicit_context() {
    let operation = IdentityOperation {
        spec: operation_spec(),
    };
    let capabilities = BTreeSet::from([HostCapability::Environment]);
    let cancellation = NeverCancelled;
    let mut context = OperationContext::new(budget(), &cancellation, capabilities.clone());
    let input = Value::Bytes(vec![0, 159, 146, 150]);

    let output = operation
        .execute(input.clone(), &Arguments::new(), &mut context)
        .expect("identity operation should execute");

    assert_eq!(operation.spec().id.as_str(), "core.identity@1");
    assert_eq!(output, input);
    assert_eq!(context.budget(), &budget());
    assert_eq!(context.capabilities(), &capabilities);
}

#[test]
fn cancellation_fails_closed_with_a_stable_code() {
    let mut context = OperationContext::new(budget(), &Cancelled, CapabilitySet::new());
    let operation = IdentityOperation {
        spec: operation_spec(),
    };

    let error = operation
        .execute(Value::Empty, &Arguments::new(), &mut context)
        .expect_err("cancelled execution must fail");

    assert_eq!(error, OperationError::Cancelled);
    assert_eq!(error.code(), "core.operation.cancelled");
}

#[test]
fn operation_error_codes_are_stable_and_specific() {
    assert_eq!(
        OperationError::InvalidArguments.code(),
        "core.operation.invalid_arguments"
    );
    assert_eq!(
        OperationError::OutputLimitExceeded.code(),
        "core.operation.output_limit_exceeded"
    );
    assert_eq!(
        OperationError::Failed {
            code: OperationFailureCode::new("encoding.invalid_padding")
                .expect("namespaced code should be valid"),
        }
        .code(),
        "encoding.invalid_padding"
    );
}

#[test]
fn operation_failure_codes_reject_ambiguous_or_unstable_values() {
    for invalid in [
        "",
        " ",
        "single",
        "Encoding.invalid_padding",
        "encoding..invalid_padding",
        "encoding.invalid!padding",
    ] {
        let error = OperationFailureCode::new(invalid).expect_err("code must be rejected");
        assert_eq!(error.code(), "core.operation.failure_code_invalid");
    }

    let maximum = format!("a.{}", "b".repeat(126));
    assert_eq!(maximum.len(), 128);
    OperationFailureCode::new(maximum).expect("maximum-length code should be accepted");

    let oversized = format!("a.{}", "b".repeat(127));
    let error = OperationFailureCode::new(oversized).expect_err("oversized code must be rejected");
    assert_eq!(error.code(), "core.operation.failure_code_invalid");
}

#[test]
fn static_failure_code_uses_the_same_validated_value() {
    assert_eq!(STATIC_FAILURE_CODE.as_str(), "encoding.hex.invalid_digit");
}

#[test]
fn value_constraints_remain_available_to_operation_implementations() {
    let spec = operation_spec();

    assert_eq!(spec.input, ValueConstraint::Any);
    assert_eq!(spec.output, ValueConstraint::Any);
}
const STATIC_FAILURE_CODE: ferrosift_core::OperationFailureCode =
    ferrosift_core::OperationFailureCode::from_static("encoding.hex.invalid_digit");
