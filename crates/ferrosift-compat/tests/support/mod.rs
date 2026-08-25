//! Real operation fixtures shared by compatibility integration tests.

use std::collections::BTreeMap;

use ferrosift_core::{Operation, OperationContext, OperationError, OperationRegistry};
use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, Arguments, CapabilitySet, ClassificationSet,
    CompatibilityAlias, CompatibilityProfile, EvidenceRecord, EvidenceState, EvidenceSummary,
    OperationId, OperationSpec, OutputBehavior, StreamingSupport, Target, TargetSet, Value,
    ValueConstraint,
};

pub struct StaticOperation {
    spec: OperationSpec,
}

impl Operation for StaticOperation {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        _context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        Ok(input)
    }
}

pub fn registry_with(
    id: &str,
    cyberchef_aliases: &[&str],
    arguments: Vec<ArgumentSpec>,
) -> OperationRegistry {
    let mut registry = OperationRegistry::new();
    registry
        .register(StaticOperation {
            spec: operation_spec(id, cyberchef_aliases, arguments),
        })
        .expect("fixture operation is valid");
    registry
}

pub fn argument(
    name: &str,
    kind: ArgumentKind,
    required: bool,
    default: Option<ArgumentValue>,
) -> ArgumentSpec {
    ArgumentSpec {
        name: name.into(),
        description: format!("Fixture argument {name}."),
        required,
        kind,
        default,
    }
}

pub fn operation_spec(
    id: &str,
    cyberchef_aliases: &[&str],
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    OperationSpec {
        id: OperationId::new(id).expect("valid operation ID"),
        display_name: id.into(),
        category: "Test".into(),
        description: "Compatibility fixture operation.".into(),
        aliases: cyberchef_aliases
            .iter()
            .map(|name| CompatibilityAlias {
                profile: CompatibilityProfile::CyberChefV11_3,
                name: (*name).into(),
            })
            .collect(),
        input: ValueConstraint::Any,
        output: ValueConstraint::Any,
        arguments,
        targets: TargetSet::from([Target::Native]),
        capabilities: CapabilitySet::new(),
        classifications: ClassificationSet::new(),
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        output_behavior: OutputBehavior::default(),
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
