//! Real operation fixtures shared by compatibility integration tests.
//!
//! Each integration test binary compiles this module separately, so a helper
//! only some of them need reads as dead in the rest.
#![allow(dead_code)]

use std::collections::BTreeMap;

use ferrosift_core::{Operation, OperationContext, OperationError, OperationRegistry};
use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, Arguments, CapabilitySet, ClassificationSet,
    CompatibilityAlias, CompatibilityProfile, EvidenceManifest, EvidenceRecord, EvidenceState,
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
    let mut registry = registry();
    registry
        .register(StaticOperation {
            spec: operation_spec(id, cyberchef_aliases, arguments),
        })
        .expect("fixture operation is valid");
    registry
}

/// A registry whose operations each carry exactly the aliases given.
///
/// Distinct from [`registry_with`], which puts every alias in 11.3 because
/// that is what almost every test needs. Profile-scoping tests need the
/// opposite: a name that exists in one profile and genuinely does not exist in
/// another, which is what an operation the reference introduced later looks
/// like.
pub type OperationFixture<'a> = (
    &'a str,
    &'a [(CompatibilityProfile, &'a str)],
    Vec<ArgumentSpec>,
);

pub fn registry_of(operations: &[OperationFixture<'_>]) -> OperationRegistry {
    let mut registry = registry();
    for (id, aliases, arguments) in operations {
        let mut spec = operation_spec(id, &[], arguments.clone());
        spec.aliases = aliases
            .iter()
            .map(|(profile, name)| CompatibilityAlias {
                profile: *profile,
                name: (*name).into(),
            })
            .collect();
        registry
            .register(StaticOperation { spec })
            .expect("fixture operation is valid");
    }
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
    }
}

/// What these fixtures claim their build checked.
pub fn manifest() -> EvidenceManifest {
    EvidenceManifest {
        provenance: enforced("fixtures/provenance"),
        license: enforced("fixtures/license"),
        conformance: enforced("fixtures/conformance"),
        benchmark: EvidenceRecord {
            state: EvidenceState::Planned,
            reference: None,
        },
        target_checks: BTreeMap::from([(Target::Native, enforced("fixtures/native"))]),
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

fn enforced(reference: &str) -> EvidenceRecord {
    EvidenceRecord {
        state: EvidenceState::Enforced,
        reference: Some(reference.into()),
    }
}
