# ferrosift-cli

The `ferrosift` command: deterministic, local-first data transformation with no
JavaScript runtime and no network access.

```bash
cargo install ferrosift-cli
```

```bash
ferrosift operations
ferrosift describe encoding.hex.encode@1
ferrosift validate --format cyberchef-v11.3 --input-kind bytes --recipe recipe.json
ferrosift run --format cyberchef-v11.4 --input-kind bytes --recipe recipe.json --input -
```

`--format` takes `ferrosift`, `cyberchef-v11.3`, or `cyberchef-v11.4`. The two
CyberChef formats parse identically — the reference's recipe model is unchanged
between those releases — and differ in which operation *names* resolve, so a
recipe using an operation 11.4 introduced loads as 11.4 and not as 11.3.

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
