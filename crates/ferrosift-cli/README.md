# ferrosift-cli

The `ferrosift` command: deterministic, local-first data transformation with no
JavaScript runtime and no network access.

```bash
cargo install ferrosift-cli
ferrosift doctor
```

Tagged GitHub releases also publish platform CLI binaries with `.sha256`
checksums. Prefer those over an unverified remote install script.
```bash
ferrosift operations
ferrosift describe encoding.hex.encode@1
ferrosift validate --format cyberchef-v11.3 --input-kind bytes --recipe recipe.json
ferrosift run --format cyberchef-v11.4 --input-kind bytes --recipe recipe.json --input -
ferrosift run --format ferrosift --input-kind bytes --recipe recipe.json --input payload.bin \
  --result-format json
ferrosift pattern validate --pattern header.hexpat
ferrosift pattern run --pattern header.hexpat --input payload.bin
ferrosift pattern validate --pattern main.hexpat --source std.io=libs/io.pat
ferrosift repro export --format cyberchef-v11.3 --input-kind bytes \
  --recipe recipe.json --input sample.bin --out-dir payload-case
ferrosift repro check --case payload-case
ferrosift candidates --input-kind bytes --input sample.bin --candidates hypotheses.json
ferrosift doctor
```

`--format` takes `ferrosift`, `cyberchef-v11.3`, or `cyberchef-v11.4`. The two
CyberChef formats parse identically — the reference's recipe model is unchanged
between those releases — and differ in which operation *names* resolve, so a
recipe using an operation 11.4 introduced loads as 11.4 and not as 11.3.

`--result-format raw` (default) writes bytes or UTF-8 text for shell pipelines.
`--result-format json` writes a tagged `ferrosift.execution.v1` envelope with
status, value, and a bounded trace, including non-byte results such as numbers
and structures. A paused breakpoint still exits non-zero after writing that
envelope.

`pattern validate` parses source only. `pattern run` evaluates against subject
bytes and writes a `ferrosift.pattern.v1` JSON tree with absolute field offsets.
Repeatable `--source SPEC=PATH` entries feed a resolver for `import` /
`#include` without an ambient filesystem in the portable crate.

`repro export` runs the recipe and writes `input.bin`, `recipe.json`,
`expected.json`, `manifest.json`, and `README.md`. The default expected origin
is `observed_only` — a regression snapshot, not independent proof. Secret-like
argument names are refused unless `--include-secrets` is set. `repro check`
replays the case in a fresh process.

`candidates` evaluates up to eight explicit recipe hypotheses on one input and
prints a `ferrosift.candidates.v1` observation table. Check counts are not
calibrated probabilities.

`doctor` runs offline install smoke checks (version, registry load, a fixed
To Hex recipe, a tiny pattern) and prints `ferrosift.doctor.v1`. Exit status is
non-zero when any check fails; the JSON report is still written first.

Every recipe is fully validated before its first step runs, so an invalid later
step cannot leave a partial effect behind. Unknown operations fail closed with
stable finding codes naming the version that was asked.

## Alpha

The compatibility claim is measured against pinned CyberChef checkouts and
replayed case by case. What is not yet settled is the command surface, which is
what the pre-release version says.

The per-operation compatibility ledger, the divergence list and the benchmark
numbers live in the
[repository](https://github.com/sergii-ziborov/ferrosift).

## Licence

Apache-2.0. FerroSift is an independent project and is not affiliated with or
endorsed by GCHQ; CyberChef is a separate project under the same licence and
Crown Copyright.
