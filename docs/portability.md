# Portability

FerroSift is `no_std` with `alloc`, and CI proves it on real bare-metal
targets rather than inferring it.

## What is gated

| Target | What it proves |
|---|---|
| `x86_64` / `aarch64` (Linux, Windows, macOS) | the full workspace, all features, all tests |
| `wasm32-unknown-unknown` | the browser and edge story |
| `thumbv7em-none-eabihf` | 32-bit ARM Cortex-M with hardware float, no OS |
| `riscv32imac-unknown-none-elf` | 32-bit RISC-V, different ABI and word layout, no OS |

`wasm32-unknown-unknown` is deliberately not treated as evidence of `no_std`:
it still ships a `std`. Only the last two rows carry that claim.

On the bare-metal targets the gate builds the facade with no features, with
`pattern`, and with `hash`, `crypto`, `text`, and `analysis` individually and
together.

## The one gap

`compression` does not build on bare metal.

The cause is a single edge: bzip2 comes from `oxiarc-bzip2`, which depends on
`oxiarc-core`, which depends on `thiserror` with default features and so
requires `std`. Nothing in FerroSift's own code needs `std` here — the
`miniz_oxide` half of the pack is bare-metal clean.

This is recorded rather than papered over. The fix is to stop borrowing that
edge: a first-party pure-Rust bzip2 codec would remove the last `std`
dependency and make every pack bare-metal, which is why it is on the roadmap
as its own piece of work rather than as a patch to a vendored manifest.

Until then, the honest statement is: **FerroSift is `no_std` on bare metal for
core, pattern, hash, crypto, text, and analysis; compression requires an
allocator plus `std` because of one transitive dependency.**

`analysis` was on that list until recently, and not for a reason of its own.
It declared `analysis = ["compression", "text"]` because recipe suggestion
probes gzip and defanged URLs, so it inherited the bzip2 edge and a regex
engine to offer two probes. Suggestion now compiles whichever probes the build
already contains: `analysis` alone offers Base64, Hex, Base32, URL, HTML, and
ROT13, and the compression and text probes appear when those packs do. A
suggester should widen with the catalog rather than decide it.

Note that the gzip and zlib *detectors* stay compiled either way. Recognising
a header is a property of the bytes, not of whether this build can act on it,
and the confidence score uses that signal even when it has no decompressor to
offer.

## Reproducing locally

```bash
rustup target add thumbv7em-none-eabihf riscv32imac-unknown-none-elf
cargo check -p ferrosift --target thumbv7em-none-eabihf --no-default-features --features hash,crypto,text,pattern
```
