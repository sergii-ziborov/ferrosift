//! Shared real operation fixtures for registry contract tests.

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ferrosift_core::{Operation, OperationContext, OperationError, OperationRegistry};
use ferrosift_model::{
    Arguments, CapabilitySet, ClassificationSet, CompatibilityAlias, CompatibilityProfile,
    EvidenceManifest, EvidenceRecord, EvidenceState, OperationId, OperationSpec, OutputBehavior,
    StreamingSupport, Target, TargetSet, Value, ValueConstraint,
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
        output_behavior: OutputBehavior::default(),
        inverse: None,
    }
}

/// What these fixtures claim their build checked.
///
/// One value for the whole test catalog, which is the point of the type: the
/// fixtures all run on the host and nowhere else, and saying so once is both
/// shorter and truer than saying it on every specification.
pub fn manifest() -> EvidenceManifest {
    EvidenceManifest {
        provenance: verified("fixtures/provenance"),
        license: verified("fixtures/license"),
        conformance: verified("fixtures/conformance"),
        benchmark: EvidenceRecord {
            state: EvidenceState::Planned,
            reference: None,
        },
        target_checks: BTreeMap::from([(Target::Native, verified("fixtures/native"))]),
    }
}

/// An empty registry already backed by [manifest].
pub fn registry() -> OperationRegistry {
    let mut registry = OperationRegistry::new();
    registry
        .declare_evidence(manifest())
        .expect("the fixture manifest must be valid");
    registry
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
