use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec,
    vec::Vec,
};

use ferrosift_model::{
    ArgumentSpec, CapabilitySet, CompatibilityAlias, CompatibilityProfile, EvidenceRecord,
    EvidenceState, EvidenceSummary, OperationClassification, OperationId, OperationSpec,
    OutputBehavior, StreamingSupport, Target, TargetSet, ValueConstraint, ValueKind,
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

/// Widens a declared input so bytes and text are interchangeable on the way in.
///
/// The reference carries one value between steps and presents it in whatever
/// type the next operation asks for, so `To Base64` twice in a row is an
/// ordinary recipe there. Declaring `Exact(Bytes)` here made it an error, which
/// was a compatibility gap rather than a safety property: nothing was being
/// protected, a legal recipe was being refused.
///
/// Widening happens on the *input* only. An output stays exactly what the
/// operation produces, because that is a fact about the operation rather than
/// a courtesy to the next step — and the type-flow preflight needs it precise
/// to say anything useful.
///
/// Every other representation is left alone. A step that wants a structured
/// value or a file list is not asking for bytes with extra steps, and
/// converting for it would be inventing a rule the reference does not have.
fn readable(declared: ValueConstraint) -> ValueConstraint {
    match declared {
        ValueConstraint::Exact(ValueKind::Bytes | ValueKind::Text) => {
            ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text]))
        }
        other => other,
    }
}

pub(crate) fn build(definition: SpecDefinition) -> OperationSpec {
    // One alias per profile the operation is proven against, not one per
    // profile that exists. Both are emitted here because `tests/profiles.rs`
    // replays every corpus case through 11.4 as well as 11.3 and refuses an
    // 11.4 alias that no 11.4 case backs — so the second entry is a claim the
    // suite has to keep earning. An operation whose behaviour genuinely
    // diverged between references would need two specs and a versioned
    // identifier instead, which that same test enforces.
    let aliases = definition
        .cyberchef_alias
        .map(|name| {
            vec![
                CompatibilityAlias {
                    profile: CompatibilityProfile::CyberChefV11_3,
                    name: String::from(name),
                },
                CompatibilityAlias {
                    profile: CompatibilityProfile::CyberChefV11_4,
                    name: String::from(name),
                },
            ]
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
        input: readable(definition.input),
        output: definition.output,
        arguments: definition.arguments,
        targets: TargetSet::from([Target::Native, Target::Wasm32UnknownUnknown]),
        capabilities: CapabilitySet::new(),
        classifications,
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        output_behavior: OutputBehavior::InputProportional,
        inverse: definition.inverse.map(operation_id),
        evidence: evidence(),
    }
}

/// Builds a spec for an operation whose output does not depend on its input.
///
/// Sequence and identifier generators read their arguments and ignore the
/// value handed to them, so the executor's expansion ratio has nothing
/// meaningful to divide by — see [`OutputBehavior::InputIndependent`]. Opting
/// in through a separate function rather than a field keeps that decision
/// visible at the one call site that makes it, instead of adding a `None` to
/// every other operation in the catalog.
///
/// The operation is still bound by the absolute output limit and by
/// cancellation, and is expected to refuse oversized requests itself.
pub(crate) fn build_generator(definition: SpecDefinition) -> OperationSpec {
    OperationSpec {
        output_behavior: OutputBehavior::InputIndependent,
        ..build(definition)
    }
}

pub(crate) const fn operation_id(value: &'static str) -> OperationId {
    OperationId::from_static(value)
}

/// A specification whose input and output are both the same value kind.
///
/// Most operations are shaped this way, and spelling out the two constraints
/// at every call site buried the parts that actually differ.
pub(crate) struct UniformSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub cyberchef_alias: &'static str,
    pub arguments: Vec<ArgumentSpec>,
}

pub(crate) fn build_uniform(kind: ValueKind, definition: UniformSpec) -> OperationSpec {
    build(SpecDefinition {
        id: definition.id,
        display_name: definition.display_name,
        category: definition.category,
        description: definition.description,
        cyberchef_alias: Some(definition.cyberchef_alias),
        input: ValueConstraint::Exact(kind),
        output: ValueConstraint::Exact(kind),
        arguments: definition.arguments,
        inverse: None,
        classifications: None,
    })
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
