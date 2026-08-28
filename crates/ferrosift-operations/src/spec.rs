use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

use ferrosift_model::{
    ArgumentSpec, CapabilitySet, CompatibilityAlias, CompatibilityProfile, EvidenceManifest,
    EvidenceRecord, OperationClassification, OperationId, OperationSpec, OutputBehavior,
    StreamingSupport, Target, TargetSet, ValueConstraint, ValueKind,
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

/// A specification for an operation that needs an operating system.
///
/// Two of them: bzip2 compress and decompress, whose crate reaches `thiserror`.
/// Everything else in the catalog builds for bare metal, and the difference
/// belongs in the spec because a caller asking "can I run this on a
/// microcontroller" is asking about the operation and not about the build.
#[cfg(feature = "compression-bzip2")]
pub(crate) fn build_hosted(definition: SpecDefinition) -> OperationSpec {
    OperationSpec {
        targets: targets(Portability::Hosted),
        ..build(definition)
    }
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
        targets: targets(Portability::BareMetal),
        capabilities: CapabilitySet::new(),
        classifications,
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        output_behavior: OutputBehavior::InputProportional,
        inverse: definition.inverse.map(operation_id),
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

/// Builds a spec for an operation whose output is a summary of fixed size.
///
/// A digest, a checksum, a distance, a statistic. The expansion ratio is the
/// wrong instrument for these in the direction nobody expects: it refuses them
/// on *small* inputs, because the output is a constant and the denominator is
/// the input. `SHA-512` of an empty input is a hundred and twenty-eight
/// characters against a denominator of one, and the generous default budget —
/// a ratio of sixty-four — refused it outright. Hashing nothing is an ordinary
/// thing to do and the reference does it happily.
///
/// The claim is checkable and is checked: `tests/output_behavior.rs` runs every
/// operation declaring this over inputs differing by a factor of two hundred
/// and fifty-six and requires the output not to grow. A misdeclaration fails
/// there rather than quietly widening what escapes the ratio.
///
/// Still bound by the absolute output limit, the total-bytes accounting and
/// cancellation, exactly as [`build_generator`] is.
pub(crate) fn build_reducer(definition: SpecDefinition) -> OperationSpec {
    OperationSpec {
        output_behavior: OutputBehavior::Reducer,
        ..build(definition)
    }
}

/// Declares that this specification's operation implements
/// [`Streamable`](ferrosift_core::Streamable).
///
/// Applied on top of whichever builder the operation already used, because
/// streaming is orthogonal to how output relates to input: a digest is a
/// reducer *and* incremental, hex encoding is proportional *and* incremental.
///
/// The declaration and the implementation must agree, and
/// `tests/streaming.rs` is what makes them: an operation declaring this and
/// offering no session fails there, and so does one whose streamed answer
/// differs from `execute`'s at any chunk size.
pub(crate) const fn incremental(mut spec: OperationSpec) -> OperationSpec {
    spec.streaming = StreamingSupport::Incremental;
    spec
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

/// What this build has checked, and where to read the check.
///
/// One value for the catalog. It used to be five records on every one of the
/// two hundred and fifty-four specifications, and reading them showed that not
/// one was a fact about an operation: the same notice file, the same licence,
/// the same workflow, `Missing` for a benchmark in a repository that publishes
/// measurements, and one test file named as the conformance evidence for every
/// operation in the catalog.
///
/// The benchmark record says `Passed` here and said `Missing` there, and both
/// were honest. As a claim about one operation it was false — most of the
/// catalog is unmeasured. As a claim about the build it is true: measurements
/// exist and are published, with the unflattering numbers in them.
///
/// Conformance points at the ledger rather than answering. How many reference
/// cases pin one operation is computed from the committed fixtures on every CI
/// run; a string in the catalog could only repeat that, and repeated it wrongly.
#[must_use]
pub(crate) fn manifest() -> EvidenceManifest {
    EvidenceManifest {
        provenance: passed("NOTICE"),
        license: passed("LICENSE"),
        conformance: passed("docs/compatibility/ledger.md"),
        benchmark: passed("docs/benchmarks.md"),
        target_checks: BTreeMap::from([
            (Target::Native, passed(".github/workflows/ci.yml")),
            (
                Target::Wasm32UnknownUnknown,
                passed(".github/workflows/ci.yml"),
            ),
            (Target::Embedded, passed(".github/workflows/ci.yml")),
        ]),
    }
}

/// Whether an operation can be built for a target without an operating system.
///
/// A property of the operation, not of the build that selected it. The first
/// attempt read `cfg!(feature = "compression-bzip2")` and so dropped `Embedded`
/// from *every* operation as soon as bzip2 was compiled anywhere — which is the
/// same mistake in reverse: SHA-256 does not stop running on a microcontroller
/// because something else in the same binary needs `std`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Portability {
    /// Builds for `thumbv7em-none-eabihf` and `riscv32imac-unknown-none-elf`.
    BareMetal,
    /// Needs an operating system. Only bzip2, which reaches `thiserror` — so a
    /// build without that pack has nothing to describe this way, and the
    /// variant is there rather than absent because the *catalog* has two states
    /// whether or not one build reaches both.
    #[cfg_attr(
        not(feature = "compression-bzip2"),
        expect(dead_code, reason = "only the bzip2 operations are hosted")
    )]
    Hosted,
}

/// Where an operation can run.
///
/// `Embedded` is here because CI proves it rather than because it sounds true:
/// the bare-metal job builds `portable-full` for both targets. The catalog used
/// to claim only the native and browser targets while the workflow had been
/// checking a third for months — a claim smaller than its evidence, which is
/// the opposite of the usual drift and just as wrong.
fn targets(portability: Portability) -> TargetSet {
    let mut targets = TargetSet::from([Target::Native, Target::Wasm32UnknownUnknown]);
    if portability == Portability::BareMetal {
        targets.insert(Target::Embedded);
    }
    targets
}

fn passed(reference: &str) -> EvidenceRecord {
    EvidenceRecord::passed(reference)
}
