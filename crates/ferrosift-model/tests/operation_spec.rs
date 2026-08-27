//! Validation and wire-format tests for operation specifications.

use std::collections::{BTreeMap, BTreeSet};

use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, CapabilitySet, ClassificationSet,
    CompatibilityAlias, CompatibilityProfile, EvidenceRecord, EvidenceState, EvidenceSummary,
    OperationId, OperationSpec, OutputBehavior, StreamingSupport, Target, TargetSet,
    ValueConstraint, ValueKind,
};

fn passed(reference: &str) -> EvidenceRecord {
    EvidenceRecord {
        state: EvidenceState::Passed,
        reference: Some(reference.into()),
    }
}

fn valid_spec() -> OperationSpec {
    let targets = TargetSet::from([Target::Native, Target::Wasm32UnknownUnknown]);
    let target_checks = BTreeMap::from([
        (Target::Native, passed("ci/native")),
        (Target::Wasm32UnknownUnknown, passed("ci/wasm")),
    ]);

    OperationSpec {
        id: OperationId::new("encoding.base64.decode@1").expect("valid operation ID"),
        display_name: "From Base64".into(),
        category: "Encoding".into(),
        description: "Decode Base64 text into bytes.".into(),
        aliases: vec![CompatibilityAlias {
            profile: CompatibilityProfile::CyberChefV11_3,
            name: "From Base64".into(),
        }],
        input: ValueConstraint::Exact(ValueKind::Text),
        output: ValueConstraint::Exact(ValueKind::Bytes),
        arguments: vec![ArgumentSpec {
            name: "alphabet".into(),
            description: "Base64 alphabet variant.".into(),
            required: false,
            kind: ArgumentKind::Text,
            default: Some(ArgumentValue::Text("standard".into())),
        }],
        targets,
        capabilities: CapabilitySet::new(),
        classifications: ClassificationSet::new(),
        deterministic: true,
        streaming: StreamingSupport::Buffered,
        output_behavior: OutputBehavior::default(),
        inverse: Some(
            OperationId::new("encoding.base64.encode@1").expect("valid inverse operation ID"),
        ),
        evidence: EvidenceSummary {
            provenance: passed("CyberChef-11.3.0/FromBase64.mjs"),
            license: passed("Apache-2.0"),
            conformance: passed("fixtures/base64.json"),
            benchmark: EvidenceRecord {
                state: EvidenceState::Planned,
                reference: Some("benchmark/base64".into()),
            },
            target_checks,
        },
    }
}

#[test]
fn complete_spec_round_trips_with_independent_evidence() {
    let spec = valid_spec();
    spec.validate().expect("complete spec should validate");

    let json = serde_json::to_string(&spec).expect("spec should serialize");
    let restored: OperationSpec = serde_json::from_str(&json).expect("spec should deserialize");

    assert_eq!(restored, spec);
    assert_eq!(restored.evidence.provenance.state, EvidenceState::Passed);
    assert_eq!(restored.evidence.license.state, EvidenceState::Passed);
    assert_eq!(restored.evidence.conformance.state, EvidenceState::Passed);
    assert_eq!(restored.evidence.benchmark.state, EvidenceState::Planned);
    assert_eq!(restored.evidence.target_checks.len(), 2);
}

#[test]
fn argument_default_must_match_its_declared_kind() {
    let mut spec = valid_spec();
    spec.arguments[0].default = Some(ArgumentValue::Integer(64));

    let error = spec.validate().expect_err("wrong default kind should fail");
    assert_eq!(
        error.code(),
        "model.operation_spec.argument_default_invalid"
    );
    assert!(error.to_string().contains("alphabet"));
}

#[test]
fn every_declared_target_requires_verified_evidence() {
    let mut spec = valid_spec();
    spec.evidence
        .target_checks
        .remove(&Target::Wasm32UnknownUnknown);

    let error = spec
        .validate()
        .expect_err("target without evidence should fail");
    assert_eq!(error.code(), "model.operation_spec.target_evidence_missing");
}

#[test]
fn at_least_one_execution_target_is_required() {
    let mut spec = valid_spec();
    spec.targets.clear();

    let error = spec
        .validate()
        .expect_err("an operation without an execution target must fail");

    assert_eq!(error.code(), "model.operation_spec.field_invalid");
}

#[test]
fn core_evidence_cannot_be_collapsed_into_an_aggregate_score() {
    let mut spec = valid_spec();
    spec.evidence.provenance = EvidenceRecord {
        state: EvidenceState::Missing,
        reference: None,
    };

    let error = spec.validate().expect_err("missing provenance should fail");
    assert_eq!(error.code(), "model.operation_spec.evidence_missing");
}

#[test]
fn duplicate_argument_names_are_rejected() {
    let mut spec = valid_spec();
    spec.arguments.push(spec.arguments[0].clone());

    let error = spec.validate().expect_err("duplicate argument should fail");
    assert_eq!(error.code(), "model.operation_spec.argument_duplicate");
}

#[test]
fn passed_evidence_requires_a_non_empty_reference() {
    let mut spec = valid_spec();
    spec.evidence.license.reference = None;

    let error = spec
        .validate()
        .expect_err("passed evidence without a reference should fail");
    assert_eq!(error.code(), "model.operation_spec.evidence_invalid");
}

#[test]
fn one_of_value_constraint_preserves_deterministic_order() {
    let constraint = ValueConstraint::OneOf(BTreeSet::from([ValueKind::Text, ValueKind::Bytes]));
    let json = serde_json::to_string(&constraint).expect("constraint should serialize");

    assert_eq!(
        serde_json::from_str::<ValueConstraint>(&json).expect("constraint should deserialize"),
        constraint
    );
}

/// The list a catalog reads to work out which profiles an alias covers.
///
/// It is a second place a profile has to be added, so what it claims about
/// itself is checked rather than assumed: everything in it is a `CyberChef`
/// release, and the releases are oldest first. The order is not cosmetic —
/// "aliased since 11.4" is resolved by comparing against it.
#[test]
fn the_cyberchef_profile_list_is_ordered_and_holds_only_cyberchef_profiles() {
    assert!(
        CompatibilityProfile::CYBERCHEF
            .iter()
            .all(|profile| profile.is_cyberchef()),
        "the list is what a catalog treats as a reference version"
    );
    assert!(
        CompatibilityProfile::CYBERCHEF
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        "profiles must be listed oldest first"
    );
    assert!(!CompatibilityProfile::Native.is_cyberchef());
}

/// An operation the reference introduced is not claimed in the versions before
/// it.
///
/// This is the whole point of scoping an alias to a range rather than to the
/// profile list: 11.4 exposes operations 11.3 has never answered to, and a spec
/// that named them in both would be asserting something false about the older
/// reference. The evidence gates cannot catch that on their own — a name the
/// reference never had has no replayed case to be missing — so what the range
/// resolves to is checked here.
#[test]
fn an_alias_is_claimed_only_from_the_profile_that_introduced_it() {
    let oldest = CompatibilityProfile::CYBERCHEF[0];
    let newest = CompatibilityProfile::CYBERCHEF[CompatibilityProfile::CYBERCHEF.len() - 1];

    let always = CompatibilityAlias::cyberchef_since(oldest, "To Base64");
    assert_eq!(always.len(), CompatibilityProfile::CYBERCHEF.len());
    assert!(always.iter().all(|alias| alias.name == "To Base64"));
    assert_eq!(always[0].profile, oldest);

    let introduced = CompatibilityAlias::cyberchef_since(newest, "Modular Exponentiation");
    assert_eq!(
        introduced,
        vec![CompatibilityAlias {
            profile: newest,
            name: "Modular Exponentiation".into(),
        }],
        "an operation introduced in the newest profile is claimed there and nowhere earlier"
    );

    assert!(
        CompatibilityAlias::cyberchef_since(CompatibilityProfile::Native, "To Base64").is_empty(),
        "FerroSift's own profile is not a reference version to claim a name in"
    );
}

#[test]
fn value_constraints_match_value_kinds() {
    let exact = ValueConstraint::Exact(ValueKind::Bytes);
    let one_of = ValueConstraint::OneOf(BTreeSet::from([ValueKind::Text, ValueKind::Bytes]));

    assert!(ValueConstraint::Any.accepts(ValueKind::Files));
    assert!(exact.accepts(ValueKind::Bytes));
    assert!(!exact.accepts(ValueKind::Text));
    assert!(one_of.accepts(ValueKind::Bytes));
    assert!(one_of.accepts(ValueKind::Text));
    assert!(!one_of.accepts(ValueKind::Structured));
}
