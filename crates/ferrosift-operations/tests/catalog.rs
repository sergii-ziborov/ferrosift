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
            "compression.gunzip@1",
            "core.identity@1",
            "data.drop_bytes@1",
            "data.head@1",
            "data.take_bytes@1",
            "encoding.base32.decode@1",
            "encoding.base32.encode@1",
            "encoding.base45.decode@1",
            "encoding.base45.encode@1",
            "encoding.base58.decode@1",
            "encoding.base58.encode@1",
            "encoding.base64.decode@1",
            "encoding.base64.encode@1",
            "encoding.base85.decode@1",
            "encoding.base85.encode@1",
            "encoding.binary.decode@1",
            "encoding.binary.encode@1",
            "encoding.decimal.decode@1",
            "encoding.decimal.encode@1",
            "encoding.hex.decode@1",
            "encoding.hex.encode@1",
            "encoding.hexdump.decode@1",
            "encoding.hexdump.encode@1",
            "encoding.octal.decode@1",
            "encoding.octal.encode@1",
            "encoding.url.decode@1",
            "encoding.url.encode@1",
            "logic.xor@1",
            "text.find_replace@1",
        ]
    );
    assert_eq!(registry.len(), 29);
}

#[test]
fn interoperability_aliases_are_exact_and_profile_scoped() {
    let registry = support::registry();
    for (alias, id) in [
        ("To Hex", "encoding.hex.encode@1"),
        ("From Hex", "encoding.hex.decode@1"),
        ("To Hexdump", "encoding.hexdump.encode@1"),
        ("From Hexdump", "encoding.hexdump.decode@1"),
        ("To Base32", "encoding.base32.encode@1"),
        ("From Base32", "encoding.base32.decode@1"),
        ("To Base45", "encoding.base45.encode@1"),
        ("From Base45", "encoding.base45.decode@1"),
        ("To Base58", "encoding.base58.encode@1"),
        ("From Base58", "encoding.base58.decode@1"),
        ("To Base64", "encoding.base64.encode@1"),
        ("From Base64", "encoding.base64.decode@1"),
        ("To Base85", "encoding.base85.encode@1"),
        ("From Base85", "encoding.base85.decode@1"),
        ("To Binary", "encoding.binary.encode@1"),
        ("From Binary", "encoding.binary.decode@1"),
        ("To Decimal", "encoding.decimal.encode@1"),
        ("From Decimal", "encoding.decimal.decode@1"),
        ("To Octal", "encoding.octal.encode@1"),
        ("From Octal", "encoding.octal.decode@1"),
        ("URL Encode", "encoding.url.encode@1"),
        ("URL Decode", "encoding.url.decode@1"),
        ("XOR", "logic.xor@1"),
        ("Gunzip", "compression.gunzip@1"),
        ("Take bytes", "data.take_bytes@1"),
        ("Drop bytes", "data.drop_bytes@1"),
        ("Head", "data.head@1"),
        ("Find / Replace", "text.find_replace@1"),
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
