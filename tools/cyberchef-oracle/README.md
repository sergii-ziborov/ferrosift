# CyberChef reference oracle

Generates the pinned fixtures that prove FerroSift's CyberChef compatibility.

This is a **development tool**. Nothing here ships: the FerroSift crates are
pure Rust with no JavaScript at build or run time, and `cargo test` replays the
generated fixtures without needing Node at all. The oracle exists so anyone can
regenerate that evidence, not only whoever already has a checkout.

## The pin

Every fixture is generated against one exact reference:

| | |
|---|---|
| Project | [CyberChef](https://github.com/gchq/CyberChef) |
| Version | 11.3.0 |
| Commit | `d24ba1afce2e3a080308b5df7db033332fe94a1a` |

The generators refuse to run against anything else. A fixture produced from a
different commit would not be evidence, so the mismatch is a hard failure
rather than a warning.

## Setup

```bash
cargo xtask cyberchef setup
```

This clones the reference at the pinned commit into `vendor/` (gitignored) and
runs `npm ci` inside it. To reuse a checkout you already have, point at it
instead:

```bash
FERROSIFT_CYBERCHEF_DIR=/path/to/CyberChef cargo xtask cyberchef verify
```

Requires Node.js and `git` on the path. Nothing else.

## Tasks

```bash
cargo xtask cyberchef generate   # rewrite both fixtures from the reference
cargo xtask cyberchef verify     # check the pin, then replay the fixtures
cargo xtask cyberchef gap        # reference operations not implemented yet
```

`gap` derives its answer from the reference catalog and from
`ferrosift operations --format json`, so the work list cannot drift from the
code the way a hand-maintained list would.

## Adding an operation

1. Implement it, giving the spec its exact `CyberChefV11_3` alias.
2. Add cases for it to `generate-corpus.mjs` — or to `generate-suite.mjs` when
   the behaviour needs a hand-picked recipe rather than sampled inputs.
3. `cargo xtask cyberchef generate`
4. `cargo test -p ferrosift-operations`

The corpus test fails if an aliased operation has neither cases nor a
documented exemption, so a new operation cannot land unverified.

A test failure at this point is the useful outcome: it reports the exact
recipe prefix where FerroSift and the reference disagree. Either fix the
implementation, or — when the reference behaviour genuinely cannot be
reproduced — record the divergence in
[`docs/compatibility/cyberchef-v11.3.0.md`](../../docs/compatibility/cyberchef-v11.3.0.md)
and exempt it explicitly. Silent divergence is the one outcome this tooling
exists to prevent.

## Files

| File | Role |
|---|---|
| `reference.mjs` | The pin, checkout resolution, and baking helpers |
| `generate-suite.mjs` | Curated recipes covering representative and quirk-prone paths |
| `generate-corpus.mjs` | Deterministically sampled cases for every aliased operation |
| `gap.mjs` | Reference operations with no FerroSift alias |

Both generators are deterministic: a seeded PRNG, no clock, no `Math.random`.
Re-running them on the same pin reproduces byte-identical fixtures.
