# ferrosift

CyberChef-compatible transformations and binary patterns in a `no_std`-first
Rust library. Native, `wasm32-unknown-unknown`, and bare metal, with no
JavaScript runtime anywhere in the build.

**Broad by design. Compatible by evidence.** Nothing here claims compatibility
it has not measured: every claim is against a pinned CyberChef v11.3.0 or
v11.4.0 checkout, and every corpus case is replayed against both. What is *not*
exact is counted rather than glossed — the
[compatibility ledger](https://github.com/sergii-ziborov/ferrosift/blob/main/docs/compatibility/ledger.md)
reports, per operation, how it was checked and how close it came.

```rust
let engine = ferrosift::Engine::portable()?;
let pipeline = engine.pipeline().from_base64().gunzip().compile(&engine)?;

for packet in packets {
    let fields = pipeline.run_pattern(FIRMWARE_HEADER, packet)?;
}
```

Transform-then-parse is one call: decode, decompress, or decrypt a buffer and
describe the bytes that come out. Compiling resolves every operation once, so a
repeated pipeline never rebuilds the registry or the recipe.

## Feature packs

The catalog is divided by what an operation *needs*, not by what it does.
Identity, flow control, byte slicing, XOR and nearly every encoding carry no
external dependency and are always compiled.

| Pack | What it adds |
|---|---|
| `hash` | Digests and HMAC |
| `crypto` | Ciphers and key derivation |
| `compression-deflate` | Deflate, gzip and zlib |
| `compression-bzip2` | Bzip2 — the one pack that is not `no_std` |
| `text` | Indicator extraction, defanging, regex find/replace |
| `arithmetic`, `bignum` | Arbitrary-precision integers and decimals |
| `analysis` | Recipe suggestion and XOR brute force |
| `pattern` | The `.hexpat` parser and evaluator, dependency-free |

`portable-full` is everything that builds on bare metal; `full` adds bzip2.

## Alpha

The compatibility claim is measured and pinned. What is not yet settled is the
API a caller programs against, which is what the pre-release version says.

Full documentation, the per-operation ledger, the divergence list and the
benchmark numbers live in the
[repository](https://github.com/sergii-ziborov/ferrosift).

## Licence

Apache-2.0. FerroSift is an independent project and is not affiliated with or
endorsed by GCHQ; CyberChef is a separate project under the same licence and
Crown Copyright.
