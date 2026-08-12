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
            "compression.bzip2.compress@1",
            "compression.bzip2.decompress@1",
            "compression.gunzip@1",
            "compression.gzip@1",
            "compression.raw.deflate@1",
            "compression.raw.inflate@1",
            "compression.zlib.deflate@1",
            "compression.zlib.inflate@1",
            "core.identity@1",
            "crypto.aes.decrypt@1",
            "crypto.aes.encrypt@1",
            "crypto.aes_kw.unwrap@1",
            "crypto.aes_kw.wrap@1",
            "crypto.pbkdf2@1",
            "crypto.rc4@1",
            "crypto.scrypt@1",
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
            "extract.file_paths@1",
            "extract.hashes@1",
            "extract.ip@1",
            "extract.mac@1",
            "extract.strings@1",
            "extract.url@1",
            "hash.hmac@1",
            "hash.md5@1",
            "hash.sha1@1",
            "hash.sha2@1",
            "hash.sha3@1",
            "logic.xor@1",
            "logic.xor_brute@1",
            "text.find_replace@1",
        ]
    );
    assert_eq!(registry.len(), 65);
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
