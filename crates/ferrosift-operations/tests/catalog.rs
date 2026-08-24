//! Built-in operation catalog contracts.

use std::collections::BTreeSet;

use ferrosift_model::{CompatibilityProfile, Target};

mod support;

/// Which operations exist is pinned by the generated compatibility ledger and
/// checked against the CLI in `ferrosift-cli/tests/discovery.rs`. This test
/// covers the properties that make that list usable: a stable order, no
/// duplicates, and identifiers in the canonical `family.name@version` form.
#[test]
fn builtin_catalog_is_complete_and_ordered() {
    let registry = support::registry();
    let ids: Vec<_> = registry
        .catalog()
        .map(|specification| specification.id.as_str())
        .collect();

    assert_eq!(ids.len(), registry.len());
    assert!(!ids.is_empty());

    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "catalog must be enumerated in sorted order");

    let unique: BTreeSet<_> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "identifiers must be unique");

    for id in &ids {
        let (path, version) = id.split_once('@').expect("id must carry a version");
        assert!(
            version.parse::<u32>().is_ok(),
            "`{id}` must end in a numeric version"
        );
        assert!(path.contains('.'), "`{id}` must be namespaced by family");
        assert!(
            path.chars().all(|value| value.is_ascii_lowercase()
                || value.is_ascii_digit()
                || value == '.'
                || value == '_'),
            "`{id}` must be lower snake case"
        );
    }
}

/// Two operations sharing a display name would make the catalog ambiguous to
/// anyone reading it, which no identifier check would catch.
#[test]
fn display_names_are_unique() {
    let registry = support::registry();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for specification in registry.catalog() {
        assert!(
            seen.insert(specification.display_name.as_str()),
            "duplicate display name `{}`",
            specification.display_name
        );
    }
}

#[test]
fn interoperability_aliases_are_exact_and_profile_scoped() {
    let registry = support::registry();
    for (alias, id) in [
        ("AES Encrypt", "crypto.aes.encrypt@1"),
        ("AES Decrypt", "crypto.aes.decrypt@1"),
        ("AES Key Wrap", "crypto.aes_kw.wrap@1"),
        ("AES Key Unwrap", "crypto.aes_kw.unwrap@1"),
        ("Bzip2 Compress", "compression.bzip2.compress@1"),
        ("Bzip2 Decompress", "compression.bzip2.decompress@1"),
        ("Raw Deflate", "compression.raw.deflate@1"),
        ("Raw Inflate", "compression.raw.inflate@1"),
        ("Derive PBKDF2 key", "crypto.pbkdf2@1"),
        ("Scrypt", "crypto.scrypt@1"),
        ("SHA3", "hash.sha3@1"),
        ("RC4", "crypto.rc4@1"),
        ("XOR Brute Force", "logic.xor_brute@1"),
        ("XOR", "logic.xor@1"),
        ("MD5", "hash.md5@1"),
        ("Extract IP addresses", "extract.ip@1"),
        ("Extract MAC addresses", "extract.mac@1"),
        ("Extract hashes", "extract.hashes@1"),
        ("Extract file paths", "extract.file_paths@1"),
        ("Fork", "flow.fork@1"),
        ("Merge", "flow.merge@1"),
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
