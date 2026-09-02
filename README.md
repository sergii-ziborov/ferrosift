# FerroSift

Reference-verified CyberChef transformations and ImHex-style binary patterns,
under explicit resource limits, anywhere Rust runs. Native,
`wasm32-unknown-unknown`, and bare metal, with no JavaScript runtime anywhere
in the build.

```toml
ferrosift = "0.1.0-alpha.1"
```

An alpha because the API is not settled, not because the claim below is. See
the [changelog](CHANGELOG.md) for what the first release contains.

Three words in that first sentence are doing the work, and each is checkable.

**Reference-verified.** Nothing here claims compatibility it has not measured.
Every claim is against a pinned CyberChef v11.3.0 or v11.4.0 checkout, every
corpus case is replayed against both at every recipe prefix, and adding the
newer profile did not retire the older — see
[reference profiles](docs/compatibility/profiles.md).

**Under explicit resource limits.** Every run carries ceilings on input,
output, expansion, steps, branches, flow depth, invocations, transient
allocation and work, and a recipe that cannot work is refused before the first
operation runs. [docs/executor.md](docs/executor.md) says exactly what is
settled before execution and what is not.

**Anywhere Rust runs.** The catalog is `no_std` + `alloc` and forbids `unsafe`,
and CI builds it for two bare-metal targets as well as for WASM and native.

And what is *not* exact is counted rather than glossed. The table below reports
two things a single word used to answer for: how much was checked, and how much
matched.

<!-- ledger:begin -->
| Registered operations | CyberChef-aliased | Differential-pinned | Exact parity | Pinned cases |
|---:|---:|---:|---:|---:|
| 254 | 252 | 248 | **230** | **6867** |
<!-- ledger:end -->

These numbers are generated, not typed: `cargo xtask ledger check` regenerates
them from the catalog and the committed fixtures on every CI run and fails if
this table disagrees.

**Exact parity** is the 230 that match the reference everywhere this project
knows of. Eighteen more are byte-pinned across their own corpus and differ
over a stated domain outside it — a reduced digest round count, an odd hex
digit, `RANDOM` padding the reference fills with `Math.random()` — and each is
listed with its domain and its reason in
[divergences.json](docs/compatibility/divergences.json). Four are compressors
whose output is one valid encoding among several, checked through the inverse
that *is* pinned. **None is aliased without evidence**, and that zero is a
build failure rather than a footnote: a coverage gate refuses an alias with
neither corpus cases nor a documented exemption.

A caller has to answer a different question before that one — **what can this
operation do?** — and the [safety matrix](docs/safety-matrix.md) is generated
from the catalog to answer it per operation: host capabilities, review
classifications, determinism, and which growth limit applies. Today no
operation requires a host capability at all, and every one of them is
deterministic; that page says so because the code does, not because anyone
wrote it down.

See the [compatibility ledger](docs/compatibility/ledger.md) for the
per-operation table and
[docs/compatibility/cyberchef-v11.3.0.md](docs/compatibility/cyberchef-v11.3.0.md)
for the argument behind each divergence.

There is also a question this README cannot answer about itself, because
everything on this page is one side of it: **should you use this instead of
something else?** [docs/comparison.md](docs/comparison.md) is that page. It
opens with the four cases where the answer is no.

The value model reproduces the reference's *conversions*, not only its
operations. A dish does not become bytes by printing itself: markup loses its
tags and entities when a later step reads it, a number prints the way
JavaScript prints one, a decimal renders through `toFixed`, and a structure
renders through `JSON.stringify(value, null, 4)`. Those differences show on the
second step of a recipe rather than the first, which is why they survived ten
operations before anything caught them.
[docs/value-model.md](docs/value-model.md) records each conversion and how it
was checked.

## Binary patterns

The same library reads `.hexpat` sources and evaluates them against bytes,
reporting the exact offset and size of every field. Measured the same way the
operations are: `ImHex`'s own `plcli`, built from a pinned checkout of
`WerWolv/PatternLanguage`, answers 104 cases covering one construct each, and
**102 of them agree**. The two that do not ask for `sizeof` of a declared type,
are held in the fixture, and are asserted to fail — so the day that changes,
the test says so.

That is a number about *constructs*, and it used to be the only one — nobody
writes one construct at a time, so it said nothing about patterns people
actually wrote. Now there is a second: every `.hexpat` in
`WerWolv/ImHex-Patterns`, the collection ImHex itself ships, parsed and
recorded per file.

**11 of 308 — 3.6%.** And the ranked reason rather than a shrug: 268 of them,
87%, stop at `import std.io;` or `#include`, because they build on ImHex's
standard library and this crate reads one source and has no filesystem to fetch
another from. That is a limit of where it runs rather than of the grammar,
which is why it carries a code of its own.

The survey paid for itself on the first run: it started at 1%, and what stood
in the way was three one-line gaps — `#pragma` metadata, `\xNN` escapes, and
`0xA000'0002` digit separators — none of them language design and none of them
visible to a corpus of hand-written constructs.

[docs/pattern-language-subset.md](docs/pattern-language-subset.md) has the
grammar, both numbers, and what is missing.

What is *not* covered on the operation side is a list rather than a number:
[operations not implemented](docs/compatibility/not-implemented.md) groups the
remaining catalog by what each one is waiting on, and says why an equivalent
Rust library is not a substitute for the one the reference used.

## Capabilities

- Representation-preserving values for bytes, encoded text, booleans,
  integers, lists, maps, and virtual files.
- Versioned recipes with stable operation and step identifiers, typed named
  arguments, metadata, disabled steps, and breakpoints.
- Validated operation contracts covering input/output values, defaults,
  execution targets, capabilities, aliases, classifications, and evidence.
- Deterministic operation registration and exact profile-scoped alias lookup.
- Preflight before execution, so an invalid later step cannot leave a partial
  effect behind. Structure, operation lookup, capability grants, arguments and
  budgets are settled before the first invocation, and so is the type flow
  along the recipe as written. What preflight cannot settle is where a jump
  sends the counter: a backward jump can present a step with a kind the
  straight-line reading never saw, and that one is refused at the step that
  received it. See [the executor's own note](docs/executor.md).
- Explicit input, output, and expansion budgets.
- Cooperative cancellation and preflight verification of declared capability
  grants; the portable core exposes no built-in host or network handles.
- Bounded execution traces containing locations and value summaries rather
  than recipe payloads.
- Streaming for the operations that can offer it, so a subject larger than the
  machine — a disk image, a firmware dump — is read a chunk at a time. Three
  today, and the reason it is trustworthy at all is that a streamed answer is
  checked against the buffered one at eight chunk sizes rather than assumed to
  match.
- Loss-aware JSON interchange for CyberChef
  [11.3.0](docs/compatibility/cyberchef-v11.3.0.md) and
  [11.4.0](docs/compatibility/profiles.md) recipes, in both directions, whose
  operations have an exact alias in the requested profile.
- Native `ferrosift` CLI with `operations`, `describe`, `validate`, and `run`
  commands for bounded file or standard-stream processing.
- An automatic differential corpus, sized in the table above: every case is
  generated by baking deterministically sampled inputs through the pinned
  reference, then replayed through the real executor and checked at **every
  recipe prefix**.

## Direction

Two capabilities, one Rust library — cyber transformations and binary
patterns:

| Layer | Crate | Status |
|---|---|---|
| Verified CyberChef-compatible operation catalog | `ferrosift-operations` | Shipping |
| Hex patterns: source → declaration tree → values with offsets | `ferrosift-pattern` | Parsing and evaluating a [documented subset](docs/pattern-language-subset.md) |
| Facade: one pipeline builder, one error type, one code space | `ferrosift` | Shipping |

Transform-then-parse is one call: decode, decompress, or decrypt a buffer and
describe the bytes that come out.

```rust
let engine = ferrosift::Engine::portable()?;
let pipeline = engine.pipeline().from_base64().gunzip().compile(&engine)?;

for packet in packets {
    let fields = pipeline.run_pattern(FIRMWARE_HEADER, packet)?;
}
```

Compiling resolves every operation once, so a repeated pipeline never rebuilds
the registry or the recipe.

## Speed

FerroSift is **not** currently faster than a best-in-class specialist crate at
anything measured. [docs/benchmarks.md](docs/benchmarks.md) says so with
numbers, prints every input size including the unflattering ones, compares
against the fastest available competitor rather than a convenient one, and
ships the raw estimates so a reader can recompute every ratio. `cargo xtask
bench all` reproduces it.

It is faster than the reference it ports, by more than an order of magnitude on
every operation and size measured — [docs/comparison.md](docs/comparison.md)
carries the range, generated from the same data rather than typed here. That is
the weaker of the two findings and the one worth less: beating a JavaScript
implementation with a Rust one is the least a port should manage. Every figure
there is a floor, computed by reading both sides as unfavourably as the data
allows, and `cargo xtask bench reference` reproduces it against the pinned
checkout.

Precision came first and it cost speed: the ports iterate characters where
bytes would do, and carry validation the specialist crates do not. Those are
reasons, not defences — the harness exists to close them, and has already
made base64 decoding 13× faster by finding a linear scan that should have
been a lookup table.

`ferrosift-pattern` now has the pinned differential corpus that claim needed —
104 cases against `ImHex`'s own runtime, 102 agreeing — but it is a corpus of
*constructs*, not of patterns people wrote. The language surface is narrower
than upstream's, so "compatible over what it implements" is the honest reading
and the subset page says what it does not implement.

## Taking only part of the catalog

`Engine::portable()` builds every operation, which is the right default and the
wrong thing for a deployment that wants three of them. Registering only what is
needed is a supported way to use this library rather than a workaround:

```rust
use ferrosift::{Engine, OperationRegistry, operations};

let mut registry = OperationRegistry::new();
registry.declare_evidence(operations::evidence_manifest())?;
registry.register(operations::FromBase64::new())?;
registry.register(operations::Gunzip::new())?;
let engine = Engine::with_registry(registry);
```

The manifest is the one line that is not obvious. A registry holds what its
build has checked — the provenance, the licence, the published measurements,
and which targets were compiled and run — and registering an operation refuses
a target claim that manifest does not cover. An operation saying it runs on
bare metal, in a registry that has checked nothing, is a claim with nothing
behind it.

The pipeline, the budgets, the traces, and `run_pattern` all work the same
against it. And the smaller catalog is not only a smaller registry: nothing
then references `default_registry`, so the operations left out can be dropped
from the binary rather than merely going unregistered.

Feature packs cut at a coarser grain -- a whole family and the dependencies it
pulls -- and the two compose: select the packs whose dependencies you accept,
then register the operations you actually call.

## Feature packs

Only what you select is compiled. Identity, flow control, every encoding,
byte slicing, and XOR carry no external dependency and are always present;
the packs below are the only ones that pull third-party crates.

| Feature | Adds | Pulls |
|---|---|---|
| `hash` | MD5, SHA-1/2/3, HMAC | RustCrypto digests |
| `crypto` | AES, key wrap, RC4, PBKDF2, scrypt | RustCrypto ciphers, KDFs |
| `compression-deflate` | gzip, zlib, raw DEFLATE | `miniz_oxide` |
| `compression-bzip2` | bzip2 | `oxiarc-bzip2` |
| `text` | extractors, defang, Find / Replace | `regex-automata` |
| `analysis` | Suggest recipe, XOR brute force | nothing |
| `bignum` | Base62, ASN.1 object identifiers, base conversion | `num-bigint` |
| `arithmetic` | Extended GCD, Modular Inverse (implies `bignum`) | `num-bigint` |
| `pattern` | the hex-pattern engine | nothing |
| `portable-full` | every pack that builds on bare metal | — |
| `full` | `portable-full` plus bzip2 | — |

The two halves of compression are named separately because only one of them is
portable. `miniz_oxide` is `no_std`; `oxiarc-bzip2` reaches `thiserror`, which
needs `std`, so a target without one can have gzip and zlib and not bzip2.
`portable-full` used to include both, which made the name a label rather than a
claim — it is now checked on `thumbv7em-none-eabihf` and
`riscv32imac-unknown-none-elf` in CI. `compression` remains as the name for
both, for a caller who wants compression and is not building for bare metal.

`default = ["full", "pattern"]`. A build that wants only binary
structure parsing costs **12 crates against the default 52**, with no cipher,
digest, compression, or regex dependency compiled at all:

```toml
ferrosift = { version = "...", default-features = false, features = ["pattern"] }
```

## Workspace

| Crate | Role |
|---|---|
| `ferrosift-model` | Portable recipe IR, values, specs, schema version |
| `ferrosift-core` | Operation trait, registry, executor, budgets, traces |
| `ferrosift-operations` | Built-in pure-Rust operations and default registry |
| `ferrosift-compat` | CyberChef 11.3 and 11.4 JSON import/export |
| `ferrosift-pattern` | Hex-pattern lexer, parser, and bounded evaluator |
| `ferrosift` | Facade: engine, pipeline builder, unified error |
| `ferrosift-cli` | Native CLI binary `ferrosift` |

Library crates are `no_std` + `alloc` and forbid `unsafe`. That claim is built
on two real bare-metal targets in CI — `thumbv7em-none-eabihf` and
`riscv32imac-unknown-none-elf` — not inferred from the WASM build, which still
has a `std`. One pack does not yet reach bare metal;
[docs/portability.md](docs/portability.md) names it and why.

## Built-in operations

<!-- catalog:begin -->
| Family | Operations |
|---|---|
| Analysis | Chi Square, Index of Coincidence, Offset checker, Suggest recipe |
| Arithmetic | Convert area, Convert data units, Convert distance, Convert mass, Convert speed, Divide, Extended GCD, MOD, Mean, Median, Modular Exponentiation, Modular Inverse, Multiply, Standard Deviation, Subtract, Sum |
| Checksums | Adler-32 Checksum, Fletcher-16 Checksum, Fletcher-32 Checksum, Fletcher-64 Checksum, Fletcher-8 Checksum, Luhn Checksum, TCP/IP Checksum, XOR Checksum |
| Ciphers | A1Z26 Cipher Decode, A1Z26 Cipher Encode, AES Decrypt, AES Encrypt, AES Key Unwrap, AES Key Wrap, Affine Cipher Decode, Affine Cipher Encode, Atbash Cipher, Bacon Cipher Decode, Bacon Cipher Encode, Bifid Cipher Decode, Bifid Cipher Encode, Caesar Box Cipher, Cetacean Cipher Decode, Cetacean Cipher Encode, Convert Leet Speak, Convert to NATO alphabet, LS47 Decrypt, LS47 Encrypt, RC4, RC4 Drop, ROT13 Brute Force, ROT47, ROT47 Brute Force, ROT8000, Rail Fence Cipher Decode, Rail Fence Cipher Encode, Substitute, TEA Decrypt, TEA Encrypt, Vigenère Decode, Vigenère Encode, XTEA Decrypt, XTEA Encrypt, XXTEA Decrypt, XXTEA Encrypt |
| Compression | Bzip2 Compress, Bzip2 Decompress, Gunzip, Gzip, LZNT1 Decompress, Raw Deflate, Raw Inflate, XPRESS Decompress, XPRESS LZ77+Huffman Decompress, Zlib Deflate, Zlib Inflate |
| Core | Identity |
| Data | Drop bytes, Drop nth bytes, Head, Remove null bytes, Reverse, Swap endianness, Take bytes, Take nth bytes |
| Defang | Defang IP Addresses, Defang URL, Fang URL |
| Distance | Hamming Distance, Levenshtein Distance |
| Encoding | Caret/M-decode, Citrix CTX1 Decode, Citrix CTX1 Encode, Decode NetBIOS Name, Encode NetBIOS Name, Escape Unicode Characters, From BCD, From Base, From Base32, From Base45, From Base58, From Base62, From Base64, From Base85, From Base92, From Bech32, From Binary, From Braille, From COBS, From Charcode, From Decimal, From Float, From HTML Entity, From Hex, From Hex Content, From Hexdump, From Modhex, From Morse Code, From Octal, From Punycode, From Quoted Printable, Microsoft Script Decoder, ROT13, Text-Integer Conversion, To BCD, To Base, To Base32, To Base45, To Base58, To Base62, To Base64, To Base85, To Base92, To Bech32, To Binary, To Braille, To COBS, To Charcode, To Decimal, To Float, To HTML Entity, To Hex, To Hex Content, To Hexdump, To Modhex, To Morse Code, To Octal, To Punycode, To Quoted Printable, URL Decode, URL Encode, Unescape Unicode Characters, Unicode Text Format, VarInt Decode, VarInt Encode |
| Extractors | Extract IP addresses, Extract MAC addresses, Extract URLs, Extract domains, Extract email addresses, Extract file paths, Extract hashes, Strings |
| Flow control | Comment, Conditional Jump, Fork, Jump, Label, Merge, Return, Subsection |
| Hashing | BLAKE2b, BLAKE2s, BLAKE3, Bcrypt parse, HMAC, Keccak, MD2, MD4, MD5, MurmurHash3, NT Hash, RIPEMD, SHA0, SHA1, SHA2, SHA3, SM3, Shake, Streebog, Whirlpool |
| KDF | Derive PBKDF2 key, Scrypt |
| Logic | ADD, AND, Bit shift left, Bit shift right, NOT, OR, Parity Bit, ROR13, Rotate left, Rotate right, SUB, XOR, XOR Brute Force |
| Networking | Change IP format, Format MAC addresses, Strip IPv4 header, Strip TCP header, Strip UDP header |
| Parsing | Hex to Object Identifier, Hex to PEM, Object Identifier to Hex, PEM to Hex, Parse TLV, Parse UNIX file permissions, Parse colour code |
| Sets | Cartesian Product, Power Set, Set Difference, Set Intersection, Set Union, Symmetric Difference |
| Shaping | Split, To Table, Unique |
| Text | Add line numbers, Alternating Caps, Count occurrences, Dechunk HTTP response, Escape Smart Characters, Expand alphabet range, Find / Replace, From Case Insensitive Regex, Generate De Bruijn Sequence, Get All Casings, HTML To Text, Pad lines, Remove ANSI Escape Codes, Remove line numbers, Remove whitespace, Strip HTML tags, Strip HTTP headers, Swap case, Tail, To Case Insensitive Regex, To Lower case, To Upper case, Unescape string, Wrap, XKCD Random Number |
| Time | UNIX Timestamp to Windows Filetime, Windows Filetime to UNIX Timestamp |
<!-- catalog:end -->

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

`--format` takes `ferrosift`, `cyberchef-v11.3`, or `cyberchef-v11.4`. The two
CyberChef formats parse identically — the reference's recipe model is unchanged
between those releases — and differ in which operation *names* resolve, so a
recipe using an operation 11.4 introduced loads as 11.4 and not as 11.3.
Unknown operations fail closed with stable finding codes naming the version
that was asked. See [reference profiles](docs/compatibility/profiles.md).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --target wasm32-unknown-unknown
```

Pinned toolchain: see `rust-toolchain.toml` (Rust 1.97).

Those four run on every push, alongside the feature-pack matrix, the two
bare-metal targets, the fuzz targets, the coverage floor, the ledger check and
`cargo package --workspace`. Two more questions need the pinned CyberChef
checkouts and so run weekly instead: whether the reference still has the names
the catalog claims, and whether regenerating the whole corpus from both
checkouts reproduces the committed fixtures byte for byte. Reproduce them with
`cargo xtask cyberchef setup --profile 11.3.0` and the commands in
[reference profiles](docs/compatibility/profiles.md).

Publishing is manual and its order matters; [docs/releasing.md](docs/releasing.md)
has the sequence.

## Compatibility and attribution

FerroSift is independent of and not endorsed by GCHQ. CyberChef is a separate
project distributed under the Apache License 2.0 and Crown Copyright.

## License

Apache-2.0.
