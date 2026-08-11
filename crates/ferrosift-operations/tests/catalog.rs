//! Built-in operation catalog contracts.

use ferrosift_model::{CompatibilityProfile, Target};

mod support;

#[test]
fn builtin_catalog_is_complete_and_ordered() {
    let registry = support::registry();
    let ids: Vec<_> = registry
        .catalog()
        .map(|specification| specification.id.as_str())
        .collect();

    assert_eq!(
        ids,
        [
            "core.identity@1",
            "encoding.base64.decode@1",
            "encoding.base64.encode@1",
            "encoding.hex.decode@1",
            "encoding.hex.encode@1",
        ]
    );
    assert_eq!(registry.len(), 5);
}

#[test]
fn interoperability_aliases_are_exact_and_profile_scoped() {
    let registry = support::registry();
    for (alias, id) in [
        ("To Hex", "encoding.hex.encode@1"),
        ("From Hex", "encoding.hex.decode@1"),
        ("To Base64", "encoding.base64.encode@1"),
        ("From Base64", "encoding.base64.decode@1"),
    ] {
        let operation = registry
            .resolve_alias(CompatibilityProfile::CyberChefV11_3, alias)
            .expect("alias must resolve");
        assert_eq!(operation.spec().id.as_str(), id);
        assert!(
            registry
                .resolve_alias(CompatibilityProfile::Native, alias)
                .is_none()
        );
    }
}

#[test]
fn every_operation_is_portable_and_host_independent() {
    let registry = support::registry();
    for specification in registry.catalog() {
        assert!(specification.targets.contains(&Target::Native));
        assert!(
            specification
                .targets
                .contains(&Target::Wasm32UnknownUnknown)
        );
        assert!(specification.capabilities.is_empty());
        assert!(specification.deterministic);
        specification
            .validate()
            .expect("specification must validate");
    }
}
