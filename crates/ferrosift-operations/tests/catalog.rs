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
            "compression.gzip@1",
            "compression.zlib.deflate@1",
            "compression.zlib.inflate@1",
            "core.identity@1",
            "data.drop_bytes@1",
            "data.head@1",
            "data.take_bytes@1",
            "defang.fang_url@1",
            "defang.ip@1",
            "defang.url@1",
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
            "encoding.charcode.decode@1",
            "encoding.charcode.encode@1",
            "encoding.decimal.decode@1",
            "encoding.decimal.encode@1",
            "encoding.hex.decode@1",
            "encoding.hex.encode@1",
            "encoding.hexdump.decode@1",
            "encoding.hexdump.encode@1",
            "encoding.html.decode@1",
            "encoding.html.encode@1",
            "encoding.octal.decode@1",
            "encoding.octal.encode@1",
            "encoding.rot13@1",
            "encoding.url.decode@1",
            "encoding.url.encode@1",
            "extract.domain@1",
            "extract.email@1",
            "extract.ip@1",
            "extract.strings@1",
            "extract.url@1",
            "hash.hmac@1",
            "hash.md5@1",
            "hash.sha1@1",
            "hash.sha2@1",
            "logic.xor@1",
            "text.find_replace@1",
        ]
    );
    assert_eq!(registry.len(), 49);
}

#[test]
fn interoperability_aliases_are_exact_and_profile_scoped() {
    let registry = support::registry();
    for (alias, id) in [
        ("Extract IP addresses", "extract.ip@1"),
        ("Extract URLs", "extract.url@1"),
        ("Extract domains", "extract.domain@1"),
        ("Extract email addresses", "extract.email@1"),
        ("Strings", "extract.strings@1"),
        ("Defang IP Addresses", "defang.ip@1"),
        ("Defang URL", "defang.url@1"),
        ("Fang URL", "defang.fang_url@1"),
        ("MD5", "hash.md5@1"),
        ("XOR", "logic.xor@1"),
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
