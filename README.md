# FerroSift

FerroSift is a pure-Rust runtime for deterministic, local-first data
transformation recipes. Its portable model and execution engine run on native
targets and `wasm32-unknown-unknown`.

The built-in **operation registry** currently exposes **53 operations** with
exact CyberChef 11.3.0 aliases where interoperability is declared.

### Ecosystem place (transform runtime — not lost)

```text
Weavatrix — code facts
Weavatrix Loom — capability Registry + compose + compile → Rust
FerroSift (this) — deterministic ops / recipes
Realforge — package / deploy artifacts
```

| FerroSift **is** | FerroSift **is not** |
| --- | --- |
| Portable recipe IR + executor + op specs | A **capability interchange Registry** (that is Loom) |
| Local-first / Wasm transform runtime | A repository indexer (that is Weavatrix) |
| Optional **source of Implementations** for Loom after conformance | A second WVX project graph |
| Something Realforge may package into larger products | Agent orchestration (Cortex) |

**Link to Loom:** a FerroSift op or recipe profile may back a Loom
`Implementation` of a `Capability` once contracts and evidence pass. Do not
merge FerroSift’s op registry into WVX IR.

Normative Loom boundaries:
[ADR-0012](https://github.com/sergii-ziborov/weavatrix-loom/blob/main/docs/adr/0012-ecosystem-boundaries.md) ·
[ecosystem distribution](https://github.com/sergii-ziborov/weavatrix-loom/blob/main/docs/ecosystem-distribution.md).

## Capabilities

- Representation-preserving values for bytes, encoded text, booleans,
  integers, lists, maps, and virtual files.
- Versioned recipes with stable operation and step identifiers, typed named
  arguments, metadata, disabled steps, and breakpoints.
- Validated operation contracts covering input/output values, defaults,
  execution targets, capabilities, aliases, classifications, and evidence.
- Deterministic operation registration and exact profile-scoped alias lookup.
- Complete preflight before execution, preventing partial side effects when a
  later step is invalid.
- Explicit input, output, and expansion budgets.
- Cooperative cancellation and preflight verification of declared capability
  grants; the portable core exposes no built-in host or network handles.
- Bounded execution traces containing locations and value summaries rather
  than recipe payloads.
- Loss-aware JSON interchange for
  [CyberChef 11.3.0 recipes](docs/compatibility/cyberchef-v11.3.0.md) whose
  operations have exact registered aliases.
- Native `ferrosift` CLI with `operations`, `describe`, `validate`, and `run`
  commands for bounded file or standard-stream processing.

## Workspace

| Crate | Role |
|---|---|
| `ferrosift-model` | Portable recipe IR, values, specs, schema version |
| `ferrosift-core` | Operation trait, registry, executor, budgets, traces |
| `ferrosift-operations` | Built-in pure-Rust operations and default registry |
| `ferrosift-compat` | CyberChef 11.3 JSON import/export |
| `ferrosift-cli` | Native CLI binary `ferrosift` |

Library crates are `no_std` + `alloc` and forbid `unsafe`.

## Built-in operations

| Family | Operations |
|---|---|
| Core | Identity |
| Analysis | Suggest recipe (Magic-as-advisor; no CyberChef Magic alias) |
| Flow control | Fork, Merge (map/join over split branches) |
| Encoding | Hex, Hexdump, Base32/45/58/64/85, Binary, Decimal, Octal, URL, HTML entities, ROT13, Charcode |
| Compression | Gzip, Gunzip, Zlib Deflate/Inflate, Raw Deflate/Inflate, Bzip2 Compress/Decompress |
| Hashing | MD5, SHA1, SHA2, SHA3, HMAC |
| Logic | XOR, XOR Brute Force |
| Ciphers | AES Encrypt/Decrypt (CBC, CFB, OFB, CTR, ECB, GCM), AES Key Wrap/Unwrap, RC4 |
| KDF | Derive PBKDF2 key, Scrypt |
| Data | Take bytes, Drop bytes, Head |
| Text | Find / Replace |
| Extractors | IP addresses, URLs, domains, emails, MAC, hashes, file paths, Strings |
| Defang | Defang IP, Defang URL, Fang URL |

Full alias tables, argument shapes, and intentional divergences from the
reference are documented in
[docs/compatibility/cyberchef-v11.3.0.md](docs/compatibility/cyberchef-v11.3.0.md).

## CLI

```bash
cargo run -p ferrosift-cli -- operations
cargo run -p ferrosift-cli -- describe encoding.hex.encode@1
cargo run -p ferrosift-cli -- validate --format cyberchef-v11.3 --input-kind bytes recipe.json
cargo run -p ferrosift-cli -- run --format cyberchef-v11.3 --input-kind bytes --recipe recipe.json
```

Recipes may be native FerroSift JSON or CyberChef 11.3 compact JSON. Unknown
CyberChef operations fail closed with stable finding codes.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --target wasm32-unknown-unknown
```

Pinned toolchain: see `rust-toolchain.toml` (Rust 1.97).

## Compatibility and attribution

FerroSift is independent of and not endorsed by GCHQ. CyberChef is a separate
project distributed under the Apache License 2.0 and Crown Copyright.

## License

Apache-2.0.
