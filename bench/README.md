# FerroSift benchmarks

A benchmark published by the project it flatters is worth nothing unless it
is arranged so that rigging it would be visible. This directory is that
arrangement.

## Running it

```bash
cargo xtask bench all
```

That measures, records the machine, and rewrites
[docs/benchmarks.md](../docs/benchmarks.md) and `docs/benchmarks.json` from
Criterion's own estimates. `cargo xtask bench run` and
`cargo xtask bench report` are the two halves if you want them separately.

## What makes it reproducible

- The toolchain is pinned here in `rust-toolchain.toml`, separately from the
  workspace root, so this directory keeps its own compiler if it is ever
  split out.
- Every comparison crate is pinned to an exact version — `=0.22.1`, not
  `0.22` — and `Cargo.lock` is committed. A benchmark that silently changes
  what it measured is not a benchmark.
- The optimisation profile is written out in `Cargo.toml` rather than
  inherited, so the numbers do not move when a cargo default does.
- Inputs come from a seeded xorshift in `src/lib.rs`. Same bytes every run,
  on every machine.
- The compiler, OS, and CPU are recorded with the results.

Timings differ between machines. Ratios should not, and the ratios are what
the report states.

## What makes it hard to rig

**The baseline is the fastest available competitor.** Where two crates do the
same job — `hex` and `faster-hex` — both are measured and the verdict is
computed against whichever is quicker. This rule exists because an earlier
run reported FerroSift beating `hex` while `faster-hex` had never been tried.

**Every size gets a row and a verdict.** There is no summary that can quote
the favourable end of a sweep.

**A ratio is only stated when the run supports it.** Overlapping confidence
intervals produce *no measurable difference*; a measurement whose own
interval exceeds 15% of its median is marked *noisy* and yields no verdict.
This cuts against FerroSift as often as for it — it was added after a loaded
machine produced a digest that appeared faster than the primitive it calls.

**FerroSift is handicapped, not helped.** It runs through its full public
surface — registry lookup, argument resolution, budget checks — and copies
its input on every iteration. Comparison crates are called through their
documented fast path with nothing changed to slow them down.

**The raw estimates are committed.** A reader who does not trust the prose
can recompute every ratio in the report from `docs/benchmarks.json`.

## Why this crate is outside the workspace

Its dependencies are comparison targets. None of them may reach the shipped
library graph, the WASM check, or the bare-metal builds — that exclusion is
what keeps "FerroSift pulls no dependency you did not ask for" checkable
rather than asserted. `xtask` is excluded for the same reason.

Being self-contained also means this directory can be split into its own
repository without changing anything but the `path` dependencies in
`Cargo.toml`.

## The peer arm

Every other comparison here is a specialist. `base64` does one thing, and
beating it would say FerroSift's codec is good. `rx-chef` is the other Rust
CyberChef port, so it answers a different question: whether a library of *this
shape* — registry, operation trait, typed arguments, pipeline — carries its
structure cheaply. Both sides pay that cost, which is what makes the
comparison about the implementations rather than the architectures.

```bash
cargo bench --features peer --bench peer
```

It is off by default, and the reason is worth recording rather than working
around. `rxchef` depends unconditionally on `fernet`, which depends on
`openssl`, which needs a system OpenSSL install and a Perl toolchain to build.
Running a benchmark should not require installing a C library, so the arm is
opt-in and the default build never pulls it.

That dependency is itself a measurement of sorts. FerroSift's portable surface
needs no system library on any target, which is the claim `docs/portability.md`
holds to two bare-metal targets. Noting where a peer differs is fair; the
numbers below, when they exist, are what actually settles anything.

Where the two disagree on output, the bench prints the divergence instead of
timing it. Comparing the speed of operations that do not produce the same bytes
is not a comparison of anything.

## Adding a comparison

1. Add the crate to `[dev-dependencies]` with an exact `=` version.
2. Add an arm named `<crate>-crate` in the relevant `benches/*.rs`, calling
   the crate's documented fastest API.
3. If it does the same job as an arm that is already there, leave both. The
   report picks the faster one as the baseline on its own.
4. Ask the input to both sides in the same shape. FerroSift's `To Hex` is
   configured for contiguous lower-case output because that is what the hex
   crates emit; a delimiter would be extra work on one side only.
