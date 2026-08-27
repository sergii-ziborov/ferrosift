use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
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

/// A specification for an operation the reference has had all along.
///
/// Almost every operation is one of these, so this is the plain form and the
/// version-scoped one is spelled out at the few call sites that need it.
pub(crate) fn build(definition: SpecDefinition) -> OperationSpec {
    build_since(CompatibilityProfile::CYBERCHEF[0], definition)
}

/// A specification for an operation the reference introduced in `earliest`.
///
/// The catalog spans more than one reference version, and until now every spec
/// asserted its name existed in all of them. That was true of all 245 aliases
/// and false of the next few: 11.4 added operations 11.3 has never heard of,
/// and claiming those in 11.3 would be claiming a name the older reference
/// cannot answer to. This takes the version the name starts existing in and
/// aliases that one and everything after it.
///
/// Opting in through a separate function rather than a field keeps the decision
/// at the call sites that make it, the same way [`build_generator`] does,
/// instead of adding an "all of them" to every other operation in the catalog.
///
/// An alias is still a claim, not a declaration. `tests/corpus.rs` and
/// `tests/profiles.rs` each refuse one that no replayed case of that profile
/// backs, so narrowing the range here narrows what has to be proven — it does
/// not exempt anything from being proven.
pub(crate) fn build_since(
    earliest: CompatibilityProfile,
    definition: SpecDefinition,
) -> OperationSpec {
    let aliases = definition
        .cyberchef_alias
        .map(|name| CompatibilityAlias::cyberchef_since(earliest, name))
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
