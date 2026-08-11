//! Real operation fixtures for bounded executor contract tests.

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use ferrosift_core::{Cancellation, ExecutionBudget, Operation, OperationContext, OperationError};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, ClassificationSet, EvidenceRecord, EvidenceState,
    EvidenceSummary, OperationId, OperationSpec, Recipe, RecipeMetadata, RecipeStep,
    StreamingSupport, Target, TargetSet, Value, ValueConstraint,
};

#[derive(Clone)]
#[allow(dead_code)]
pub enum Behavior {
    Identity,
    Append(Vec<u8>),
    AppendArgument(&'static str),
    Return(Value),
    Fail(OperationError),
    Signal(Arc<AtomicBool>),
}

pub struct FixtureOperation {
    pub spec: OperationSpec,
    pub invocations: Arc<AtomicUsize>,
    pub behavior: Behavior,
}

impl Operation for FixtureOperation {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        self.invocations.fetch_add(1, Ordering::SeqCst);

        match &self.behavior {
            Behavior::Identity => Ok(input),
            Behavior::Append(suffix) => append(input, suffix),
            Behavior::AppendArgument(name) => {
                let Some(ArgumentValue::Bytes(suffix)) = arguments.get(*name) else {
                    return Err(OperationError::InvalidArguments);
                };
                append(input, suffix)
            }
            Behavior::Return(value) => Ok(value.clone()),
            Behavior::Fail(error) => Err(error.clone()),
            Behavior::Signal(flag) => {
                flag.store(true, Ordering::SeqCst);
                Ok(input)
            }
        }
    }
}

fn append(input: Value, suffix: &[u8]) -> Result<Value, OperationError> {
    let Value::Bytes(mut bytes) = input else {
        return Err(OperationError::InvalidArguments);
    };
    bytes.extend_from_slice(suffix);
    Ok(Value::Bytes(bytes))
}

#[allow(dead_code)]
pub struct AtomicCancellation {
    pub cancelled: Arc<AtomicBool>,
}

impl Cancellation for AtomicCancellation {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub fn counter() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}

pub fn operation(id: &str, behavior: Behavior, invocations: Arc<AtomicUsize>) -> FixtureOperation {
    FixtureOperation {
        spec: spec(id),
        invocations,
        behavior,
    }
}

pub fn spec(id: &str) -> OperationSpec {
    OperationSpec {
        id: OperationId::new(id).expect("valid operation ID"),
        display_name: id.into(),
        category: "Test".into(),
        description: "Executor contract fixture.".into(),
        aliases: Vec::new(),
        input: ValueConstraint::Any,
        output: ValueConstraint::Any,
        arguments: Vec::new(),
        targets: TargetSet::from([Target::Native]),
        capabilities: CapabilitySet::new(),
        classifications: ClassificationSet::new(),
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        inverse: None,
        evidence: EvidenceSummary {
            provenance: verified("fixtures/provenance"),
            license: verified("fixtures/license"),
            conformance: verified("fixtures/conformance"),
            benchmark: EvidenceRecord {
                state: EvidenceState::Planned,
                reference: None,
            },
            target_checks: BTreeMap::from([(Target::Native, verified("fixtures/native"))]),
        },
    }
}

fn verified(reference: &str) -> EvidenceRecord {
    EvidenceRecord {
        state: EvidenceState::Passed,
        reference: Some(reference.into()),
    }
}

pub fn step(id: &str, operation: &str) -> RecipeStep {
    RecipeStep {
        id: ferrosift_model::StepId::new(id).expect("valid step ID"),
        operation: OperationId::new(operation).expect("valid operation ID"),
        arguments: Arguments::new(),
        disabled: false,
        breakpoint: false,
    }
}

pub fn recipe(steps: Vec<RecipeStep>) -> Recipe {
    Recipe::new(steps, RecipeMetadata::default()).expect("valid recipe")
}

pub const fn budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 16,
        max_input_bytes: 1_024,
        max_output_bytes: 1_024,
        max_expansion_ratio: 16,
    }
}
