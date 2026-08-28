# Releasing

The whole workspace shares one version and is released together. The crates
reference each other by exact version, so a release is all seven or none:
publishing `ferrosift-operations` against a `ferrosift-core` that had moved
would ship a combination nothing here has ever built.

Publishing is deliberately manual. CI proves the workspace *can* be packaged on
every push — the `Packaging` job runs `cargo package --workspace`, which builds
each crate from its own unpacked copy — but uploading is an irreversible,
outward-facing act and belongs to a person with a token, not to a workflow.

## Before

1. **Everything green.** The four local gates, plus the two that need the
   pinned checkouts:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo check --workspace --target wasm32-unknown-unknown
   cargo xtask ledger check
   cargo xtask cyberchef gap --profile 11.3.0 --check
   cargo xtask cyberchef gap --profile 11.4.0 --check
   ```

2. **The version, in one place.** `version` under `[workspace.package]` in the
   root `Cargo.toml`, and the same string in the five entries under
   `[workspace.dependencies]`. Nothing else carries it: `bench`, `fuzz` and
   `xtask` stay at `0.0.0` because they are not published.

3. **The changelog.** Move `Unreleased` into a dated section and add the
   comparison links at the bottom.

4. **A dry run**, which is what CI already does plus the upload check:

   ```bash
   cargo publish --workspace --dry-run
   ```

## Publishing

```bash
cargo publish --workspace
```

One command, and cargo works out the order — `ferrosift-model` first, then
`ferrosift-core`, `ferrosift-compat`, `ferrosift-operations`,
`ferrosift-pattern`, `ferrosift`, and `ferrosift-cli`. It waits for the index
between them, so a member is never uploaded before the version it names is
resolvable.

Then tag what was published:

```bash
git tag -a v0.1.0-alpha.1 -m "FerroSift 0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

## What ships

`ferrosift-operations` carries the full 11.3 corpus, which is most of its four
megabytes. Excluding it was the first thing tried and it is the wrong trade: a
package whose own `cargo test` cannot compile stops being checkable the moment
it leaves the repository, and that corpus is the evidence behind every claim
the crate makes. Compressed it comes to about six hundred kilobytes.

The 11.4 corpus is *not* in the tree at all — it is reconstructed at test time
from the baseline plus a committed overlay, which is why a second reference
costs a hundred and fifty kilobytes rather than another four megabytes.

## Yanking

A published version cannot be replaced, only yanked, and a yank leaves existing
lockfiles working while stopping new ones from selecting it:

```bash
cargo yank --version 0.1.0-alpha.1 ferrosift
```

Yank the whole set, in the reverse of the publishing order, or a dependent is
left pointing at a version nothing else can reach.
