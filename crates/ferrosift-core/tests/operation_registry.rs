//! Contract tests for atomic operation registration and lookup.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ferrosift_core::RegistryError;
use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, CompatibilityProfile, OperationId,
};

#[path = "support/registry.rs"]
mod registry_support;

use registry_support::{StaticOperation, SwitchingOperation, alias, operation, spec};

#[test]
fn lookup_and_catalog_are_deterministic() {
    let mut registry = registry_support::registry();
    registry
        .register(operation(
            "encoding.hex.decode@1",
            vec![alias(CompatibilityProfile::CyberChefV11_3, "From Hex")],
        ))
        .expect("first operation should register");
    registry
        .register(operation(
            "encoding.base64.decode@1",
            vec![alias(CompatibilityProfile::CyberChefV11_3, "From Base64")],
        ))
        .expect("second operation should register");

    let base64_id = OperationId::new("encoding.base64.decode@1").expect("valid id");
    assert_eq!(
        registry.get(&base64_id).expect("lookup by id").spec().id,
        base64_id
    );
    assert_eq!(
        registry
            .resolve_alias(CompatibilityProfile::CyberChefV11_3, "From Hex")
            .expect("lookup by alias")
            .spec()
            .id
            .as_str(),
        "encoding.hex.decode@1"
    );

    let ids: Vec<_> = registry
        .catalog()
        .map(|operation_spec| operation_spec.id.as_str())
        .collect();
    assert_eq!(ids, ["encoding.base64.decode@1", "encoding.hex.decode@1"]);
}

#[test]
fn registry_exposes_the_validated_spec_snapshot() {
    let use_alternate = Arc::new(AtomicBool::new(false));
    let registered_id = OperationId::new("core.registered@1").expect("valid id");
    let operation = SwitchingOperation {
        registered: spec(
            registered_id.as_str(),
            vec![alias(CompatibilityProfile::Native, "Registered")],
        ),
        alternate: spec(
            "core.alternate@1",
            vec![alias(CompatibilityProfile::Native, "Alternate")],
        ),
        use_alternate: Arc::clone(&use_alternate),
    };
    let mut registry = registry_support::registry();
    registry.register(operation).expect("valid initial spec");

    use_alternate.store(true, Ordering::SeqCst);

    assert_eq!(
        registry
            .get(&registered_id)
            .expect("registered id")
            .spec()
            .id,
        registered_id
    );
    assert_eq!(
        registry.catalog().next().expect("catalog entry").id,
        registered_id
    );
    assert!(
        registry
            .resolve_alias(CompatibilityProfile::Native, "Alternate")
            .is_none()
    );
}

#[test]
fn duplicate_ids_fail_without_mutating_the_registry() {
    let mut registry = registry_support::registry();
    registry
        .register(operation("core.identity@1", Vec::new()))
        .expect("first operation should register");

    let error = registry
        .register(operation("core.identity@1", Vec::new()))
        .expect_err("duplicate id must fail");

    assert_eq!(error.code(), "core.registry.operation_duplicate");
    assert_eq!(registry.len(), 1);
}

#[test]
fn duplicate_aliases_fail_without_partial_insertion() {
    let mut registry = registry_support::registry();
    registry
        .register(operation(
            "encoding.hex.decode@1",
            vec![alias(CompatibilityProfile::CyberChefV11_3, "Decode")],
        ))
        .expect("first operation should register");

    let error = registry
        .register(operation(
            "encoding.base64.decode@1",
            vec![alias(CompatibilityProfile::CyberChefV11_3, "Decode")],
        ))
        .expect_err("same profile alias must fail");

    assert_eq!(error.code(), "core.registry.alias_duplicate");
    assert_eq!(registry.len(), 1);
    assert!(
        registry
            .get(&OperationId::new("encoding.base64.decode@1").expect("valid id"))
            .is_none()
    );
}

#[test]
fn duplicate_aliases_inside_one_spec_fail_before_insertion() {
    let duplicate = alias(CompatibilityProfile::Native, "Decode");
    let mut registry = registry_support::registry();

    let error = registry
        .register(operation(
            "core.decode@1",
            vec![duplicate.clone(), duplicate],
        ))
        .expect_err("candidate-local alias collision must fail");

    assert_eq!(error.code(), "core.registry.alias_duplicate");
    assert!(registry.is_empty());
}

#[test]
fn identical_alias_text_is_allowed_in_different_profiles() {
    let mut registry = registry_support::registry();
    registry
        .register(operation(
            "core.decode.native@1",
            vec![alias(CompatibilityProfile::Native, "Decode")],
        ))
        .expect("native alias should register");
    registry
        .register(operation(
            "core.decode.cyberchef@1",
            vec![alias(CompatibilityProfile::CyberChefV11_3, "Decode")],
        ))
        .expect("profile-scoped alias should register");

    assert_eq!(registry.len(), 2);
    assert_eq!(
        registry
            .resolve_alias(CompatibilityProfile::Native, "Decode")
            .expect("native alias")
            .spec()
            .id
            .as_str(),
        "core.decode.native@1"
    );
}

#[test]
fn invalid_argument_defaults_preserve_the_specific_error_code() {
    let mut invalid = spec("core.invalid@1", Vec::new());
    invalid.arguments.push(ArgumentSpec {
        name: "alphabet".into(),
        description: "Selected alphabet.".into(),
        required: false,
        kind: ArgumentKind::Text,
        default: Some(ArgumentValue::Integer(64)),
    });
    let mut registry = registry_support::registry();

    let error = registry
        .register(StaticOperation { spec: invalid })
        .expect_err("invalid specification must fail");

    assert!(matches!(error, RegistryError::InvalidSpec(_)));
    assert_eq!(
        error.code(),
        "model.operation_spec.argument_default_invalid"
    );
    assert_eq!(
        error.to_string(),
        "model.operation_spec.argument_default_invalid: alphabet"
    );
    assert!(registry.is_empty());
}

/// An operation may not claim a target this build has not checked.
///
/// The claim and the check used to live in the same struct, so this could only
/// catch a specification disagreeing with a copy of itself. They are two values
/// now: the specification says where the operation runs, the registry's
/// manifest says what the build compiled and ran, and registering compares them.
#[test]
fn missing_target_evidence_preserves_the_specific_error_code() {
    let mut manifest = registry_support::manifest();
    manifest.target_checks.clear();
    let mut registry = ferrosift_core::OperationRegistry::new();
    registry
        .declare_evidence(manifest)
        .expect("a manifest that checked no target is still a valid manifest");

    let error = registry
        .register(registry_support::operation("core.unverified@1", Vec::new()))
        .expect_err("unverified target must fail");

    assert!(matches!(error, RegistryError::InvalidSpec(_)));
    assert_eq!(error.code(), "model.operation_spec.target_evidence_missing");
    assert!(registry.is_empty());
}

/// An empty registry claims nothing, so it backs nothing.
///
/// The default is not a convenience that lets an unevidenced catalog through —
/// it is a manifest with nothing in it, and every operation declares at least
/// one target.
#[test]
fn a_registry_that_declared_no_evidence_registers_nothing() {
    let mut registry = ferrosift_core::OperationRegistry::new();
    let error = registry
        .register(registry_support::operation("core.unbacked@1", Vec::new()))
        .expect_err("a registry with no evidence must back no operation");

    assert_eq!(error.code(), "model.operation_spec.target_evidence_missing");
    assert!(registry.is_empty());
    assert!(!registry.evidence().covers(ferrosift_model::Target::Native));
}

/// Evidence may not be narrowed under operations that already rely on it.
///
/// Declaring a manifest after registering is allowed — the order is the
/// caller's — but a manifest that no longer covers what is already registered
/// is refused, and the registry keeps the one it had.
#[test]
fn evidence_cannot_be_replaced_with_less_than_the_catalog_needs() {
    let mut registry = registry_support::registry();
    registry
        .register(registry_support::operation("core.backed@1", Vec::new()))
        .expect("a covered operation registers");

    let mut narrowed = registry_support::manifest();
    narrowed.target_checks.clear();
    let error = registry
        .declare_evidence(narrowed)
        .expect_err("narrowing evidence under a registered operation must fail");

    assert_eq!(error.code(), "model.operation_spec.target_evidence_missing");
    assert!(
        registry.evidence().covers(ferrosift_model::Target::Native),
        "the refused manifest must not have been applied"
    );
}
