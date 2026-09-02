# Changelog

Notable changes to FerroSift. The whole workspace shares one version and is
released together: the crates reference each other by exact version, so a
release is all of them or none.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
from `0.1.0` onward.

## [Unreleased]

### Added

- **Flow control is complete**, and the executor has a program counter rather
  than a cursor. `Jump`, `Conditional Jump`, `Return` and `Subsection` join
  `Fork`, `Merge`, `Label` and `Comment`, which is all seven of the reference's.
  A step now answers a second question — where execution goes next — through
  `FlowDirective`, which is what lets `Conditional Jump` evaluate its own
  regular expression and `Subsection` report byte spans while `ferrosift-core`
  compiles no pattern at all.
- **The reference's own interpreter as an oracle.** CyberChef's Node API
  refuses flow control outright and does not export `Label` or `Comment` as
  functions, which is why `Fork`, `Merge`, `Label` and `Comment` were exempt
  from the corpus rather than in it. That is the Node wrapper's restriction and
  not the reference's: the oracle bakes through its `Recipe` class instead — the
  code path the browser uses — so all seven are pinned against real reference
  bytes at every recipe prefix, and the exemption list is four entries shorter.

- **`docs/comparison.md`** — what FerroSift is an alternative *to*, opening with
  the four cases where the answer is to use something else. Its figures are
  generated from the committed fixtures and a stale one fails CI; facts about
  the other projects are constants with the revision they were read at, and
  nothing else is graded on axes this repository has not measured for it.
- **Clusters.** `OperationId::cluster` names the grouping the identifiers
  already carried — the namespace an encoder shares with its decoder — and
  `tests/clusters.rs` holds it to rules nothing checked before: a declared
  inverse must exist, must be declared back, and must live in the same cluster.
  The first run found the catalog's one asymmetric inverse. The cluster is a
  column in the ledger, a summary above it, and a field in the catalog JSON,
  which also gains `inverse`.
- **Blocker classes in the backlog.** `cargo xtask cyberchef gap` now says what
  actually stands in the way of each missing operation rather than which import
  stands in for it, grouped so the fifteen blocked by nothing but work are not
  buried among two hundred and fifty-two. Two answers the import grouping could
  not give: nine operations can never be byte-pinned by anyone, because their
  output embeds a random value or the current time, and fifty-six answer with a
  rendering rather than with bytes. The counts are published on
  `not-implemented.md` and `gap --check` refuses a stale one.

### Changed

- **Evidence is a property of the build, and there is one of it.**
  `OperationSpec.evidence` held five records on every one of the 254
  specifications, and not one dimension of it was a fact about an operation: the
  same notice file, the same licence, the same workflow, `Missing` for a
  benchmark in a repository that publishes measurements — and one test file
  named as the conformance evidence for the whole catalog, which was false for
  253 of them. `EvidenceManifest` replaces it, the registry holds one, and
  `OperationRegistry::declare_evidence` is where a catalog says what stands
  behind it. **Breaking:** `OperationSpec` loses its `evidence` field,
  `EvidenceSummary` is gone, and a registry must declare a manifest before
  registering anything.
- The invariant that survived is stronger than the one it replaces. An
  operation's declared targets used to be checked against a copy of the target
  checks stored beside them, so it could only catch a specification disagreeing
  with itself. They are checked against the manifest now: an operation may not
  claim a target this build did not compile and run.
- `ferrosift operations --format json` publishes the manifest, so a reviewer
  asking "what backs this catalog?" has somewhere to look. Targets in that JSON
  are serialized through `serde` rather than `Debug`, so they read the same way
  as the manifest keys beside them (`wasm32_unknown_unknown`, not
  `Wasm32UnknownUnknown`).

### Fixed

- A step declaring `Any` on both sides is transparent to the cross-step type
  check. It used to carry `Any` forward as "the next step might receive any
  kind", and the check then demanded that *every* kind flow — including the
  three with no byte form. `Identity`, `Comment` or `Label` in front of a step
  that wanted text was refused before the first invocation: a legal recipe
  rejected by a question that could not be answered yes. A genuine mismatch is
  now caught at the step that received it.
- `OperationRegistry` forwards every `Operation` method to the registered
  implementation. It forwarded `execute` and nothing else, so a trait method
  with a default silently answered for the operation — which is how a
  registered `Jump` reported "continue with the next step".

## [0.1.0-alpha.1]

The first published release, and a pre-release on purpose. The compatibility
claim is measured and pinned; what is not yet settled is the API a caller
programs against, which is what `alpha` says in the one place a dependency
resolver reads.

### The claim

| | |
|---:|---|
| **250** | registered operations |
| **248** | with a CyberChef alias |
| **240** | differential-pinned against a reference checkout |
| **227** | exact parity |
| **6,730** | pinned cases, replayed at every recipe prefix |
| **0** | aliased with no evidence |

Seventeen operations carry a *documented divergence*: byte-pinned across their
own corpus and differing over a stated domain outside it, each listed with its
domain and reason in `docs/compatibility/divergences.json`. Four are
compressors whose output is one valid encoding among several, checked through
the inverse that is pinned. The last number is a build failure rather than a
footnote — a coverage gate refuses an alias with neither corpus cases nor a
documented exemption.

### Added

- **The operation catalog** (`ferrosift-operations`), divided into feature
  packs by what an operation *needs* rather than by what it does. Identity,
  flow control, byte slicing, XOR and nearly every encoding carry no external
  dependency and are always compiled.
- **Two reference profiles.** A compatibility claim is against a *version*, not
  against a project. CyberChef 11.3.0 is stored in full and 11.4.0 as a delta
  against it, and both are replayed — adding the newer one did not retire the
  older. All three operations 11.4 introduced are ported: Modular
  Exponentiation, XPRESS Decompress, and XPRESS LZ77+Huffman Decompress.
- **Recipe interchange in both directions** (`ferrosift-compat`), for
  `cyberchef-v11.3` and `cyberchef-v11.4`. One parser, because the reference's
  recipe model is byte-identical between those releases; the profile decides
  which operation *names* resolve, and export refuses rather than writing a
  name the requested version cannot load.
- **The value model**, which reproduces the reference's *conversions* and not
  only its operations. Markup loses its tags when a later step reads it, a
  number prints the way JavaScript prints one, a decimal renders through
  `toFixed`, and a structure through `JSON.stringify(value, null, 4)`.
- **Explicit execution budgets** covering input, output, expansion ratio,
  steps, branches, flow depth, invocations, transient allocation and work
  units — with complete preflight, so an invalid later step cannot leave a
  partial effect behind.
- **The hex-pattern engine** (`ferrosift-pattern`): a `.hexpat` parser and
  evaluator that reports the exact offset and size of every field, with no
  third-party dependency at all. Measured against ImHex's own `plcli` from a
  pinned checkout: 104 cases, 102 agreeing, and the two that do not are held in
  the fixture and asserted to fail.
- **One pipeline API** (`ferrosift`) that resolves every operation once, so a
  repeated pipeline never rebuilds the registry or the recipe.
- **The `ferrosift` command** (`ferrosift-cli`) with `operations`, `describe`,
  `validate` and `run`.

### Portability

`no_std` is a claim about targets that have no operating system, and
`wasm32-unknown-unknown` is not one of them. Every pack but one is built for
`thumbv7em-none-eabihf` and `riscv32imac-unknown-none-elf` in CI. The exception
is `compression-bzip2`, which reaches `thiserror` and therefore `std`; it is
named separately rather than folded into `portable-full`, which now means what
it says.

### How the claim is kept

- Every corpus case is generated by baking a deterministically sampled input
  through the pinned reference and is replayed at **every recipe prefix**, so a
  divergence is reported at the step that caused it.
- `cargo xtask ledger check` regenerates the published numbers from the catalog
  and the committed fixtures on every CI run and fails if they disagree.
- A weekly job asks the two questions the committed tree cannot: whether the
  reference still has the names the catalog claims, and whether regenerating
  the whole corpus from both checkouts reproduces the committed fixtures byte
  for byte.
- Ten fuzz targets, four of which assert a property rather than only looking
  for a panic, with committed seed corpora where a mutator would not otherwise
  reach the interesting inputs.
- A coverage floor rather than a target: whatever the suite already covers, it
  does not lose.

### Known limitations

- 256 of CyberChef 11.3.0's 501 operations are not implemented.
  `docs/compatibility/not-implemented.md` groups them by what each is waiting
  on and says why an equivalent Rust library is not a substitute for the one
  the reference used.
- The pattern language is a documented *subset* of upstream's. The corpus
  covers constructs rather than patterns people wrote;
  `docs/pattern-language-subset.md` says what is missing.
- FerroSift is not currently faster than a best-in-class specialist crate at
  anything measured. `docs/benchmarks.md` says so with numbers, including the
  unflattering ones.

[Unreleased]: https://github.com/sergii-ziborov/ferrosift/compare/v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/sergii-ziborov/ferrosift/releases/tag/v0.1.0-alpha.1
