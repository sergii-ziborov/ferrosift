use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};

use ferrosift_model::{
    ArgumentSpec, CapabilitySet, CompatibilityAlias, CompatibilityProfile, EvidenceRecord,
    EvidenceState, EvidenceSummary, OperationClassification, OperationId, OperationSpec,
    StreamingSupport, Target, TargetSet, ValueConstraint,
};

pub(crate) struct SpecDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub cyberchef_alias: Option<&'static str>,
    pub input: ValueConstraint,
    pub output: ValueConstraint,
    pub arguments: Vec<ArgumentSpec>,
    pub inverse: Option<&'static str>,
    /// Optional review classifications; omit with `None` for ordinary ops.
    pub classifications: Option<&'static [OperationClassification]>,
}

pub(crate) fn build(definition: SpecDefinition) -> OperationSpec {
    let aliases = definition
        .cyberchef_alias
        .map(|name| {
            vec![CompatibilityAlias {
                profile: CompatibilityProfile::CyberChefV11_3,
                name: String::from(name),
            }]
        })
        .unwrap_or_default();
    let classifications = definition
        .classifications
        .unwrap_or(&[])
        .iter()
        .copied()
        .collect();

    OperationSpec {
        id: operation_id(definition.id),
        display_name: String::from(definition.display_name),
        category: String::from(definition.category),
        description: String::from(definition.description),
        aliases,
        input: definition.input,
        output: definition.output,
        arguments: definition.arguments,
        targets: TargetSet::from([Target::Native, Target::Wasm32UnknownUnknown]),
        capabilities: CapabilitySet::new(),
        classifications,
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        inverse: definition.inverse.map(operation_id),
        evidence: evidence(),
    }
}

pub(crate) const fn operation_id(value: &'static str) -> OperationId {
    OperationId::from_static(value)
}

fn evidence() -> EvidenceSummary {
    EvidenceSummary {
        provenance: passed("NOTICE"),
        license: passed("LICENSE"),
        conformance: passed("crates/ferrosift-operations/tests/conformance.rs"),
        benchmark: EvidenceRecord {
            state: EvidenceState::Missing,
            reference: None,
        },
        target_checks: BTreeMap::from([
            (Target::Native, passed(".github/workflows/ci.yml")),
            (
                Target::Wasm32UnknownUnknown,
                passed(".github/workflows/ci.yml"),
            ),
        ]),
    }
}

fn passed(reference: &str) -> EvidenceRecord {
    EvidenceRecord {
        state: EvidenceState::Passed,
        reference: Some(String::from(reference)),
    }
}
