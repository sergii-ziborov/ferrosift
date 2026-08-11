//! Shared real operation fixtures for registry contract tests.

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, CapabilitySet, ClassificationSet, CompatibilityAlias, CompatibilityProfile,
    EvidenceRecord, EvidenceState, EvidenceSummary, OperationId, OperationSpec, StreamingSupport,
    Target, TargetSet, Value, ValueConstraint,
};

pub struct StaticOperation {
    pub spec: OperationSpec,
}

pub struct SwitchingOperation {
    pub registered: OperationSpec,
    pub alternate: OperationSpec,
    pub use_alternate: Arc<AtomicBool>,
}

impl Operation for StaticOperation {
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

impl Operation for SwitchingOperation {
    fn spec(&self) -> &OperationSpec {
        if self.use_alternate.load(Ordering::SeqCst) {
            &self.alternate
        } else {
            &self.registered
        }
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

fn verified(reference: &str) -> EvidenceRecord {
    EvidenceRecord {
        state: EvidenceState::Passed,
        reference: Some(reference.into()),
    }
}

pub fn spec(id: &str, aliases: Vec<CompatibilityAlias>) -> OperationSpec {
    OperationSpec {
        id: OperationId::new(id).expect("valid operation id"),
        display_name: id.into(),
        category: "Test".into(),
        description: "Registry contract fixture.".into(),
        aliases,
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

pub fn alias(profile: CompatibilityProfile, name: &str) -> CompatibilityAlias {
    CompatibilityAlias {
        profile,
        name: name.into(),
    }
}

pub fn operation(id: &str, aliases: Vec<CompatibilityAlias>) -> StaticOperation {
    StaticOperation {
        spec: spec(id, aliases),
    }
}
