# Releasing

The whole workspace shares one version and is released together. The crates
reference each other by exact version, so a release is all seven or none:
publishing `ferrosift-operations` against a `ferrosift-core` that had moved
would ship a combination nothing here has ever built.

Publishing is a deliberate act with an audit trail. Pushing a tag is the act;
`.github/workflows/release.yml` is the checklist, and it cannot skip a step
because somebody was in a hurry.

This is a change from the older arrangement, which said uploading belongs to a
person with a token and not to a workflow. The tag is still the person's; what
moved is where the token lives and whether the gates are re-run against the
exact commit being published. A laptop that happened to have an unrelated file
open is a worse place for both.

## Before

1. **Everything green locally.** One command runs the cheap gates the release
   workflow re-runs, plus the three manifests `--all` cannot see:

   ```bash
   cargo xtask ci check
   ```

   And the two that need the pinned reference checkouts, which the workflow
   does not run because cloning two references takes longer than a release
   should:

   ```bash
   cargo xtask cyberchef gap --profile 11.3.0 --check
   cargo xtask cyberchef gap --profile 11.4.0 --check
   ```

2. **The version, in one place.** `version` under `[workspace.package]` in the
   root `Cargo.toml`, and the same string in the five entries under
   `[workspace.dependencies]`. Nothing else carries it: `bench`, `fuzz` and
   `xtask` stay at `0.0.0` because they are not published. The workflow refuses
   a tag that does not match this string, before it runs anything else.

3. **The changelog.** Move `Unreleased` into a dated section and add the
   comparison links at the bottom. The release notes are cut from that section
   by heading, so the version must appear as `## [x.y.z]`.

4. **A rehearsal.** Run the workflow by hand from the Actions tab with
   `dry_run` left on. It verifies everything and stops before uploading.

## Publishing

```bash
git tag -a v0.1.0-alpha.1 -m "FerroSift 0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

The workflow then re-runs every gate against that exact commit, records what it
cleared in `build-attestation.json`, runs `cargo publish --workspace`, and
creates the GitHub release with the attestation attached.

The order is `ferrosift-model` first, then `ferrosift-core`,
`ferrosift-compat`, `ferrosift-operations`, `ferrosift-pattern`, `ferrosift`,
and `ferrosift-cli`. Cargo waits for the index between them, so a member is
never uploaded before the version it names is resolvable.

### Why it publishes one crate at a time

`cargo publish --workspace` is the obvious command and it is the wrong one for a
*first* release. crates.io limits how many **new** crates an account may create
in a window, and seven trips it: on the first attempt four went up, the fifth
got `429 Too Many Requests`, and the command stopped. Re-running failed
differently — `crate ferrosift-core@0.1.0-alpha.1 already exists on crates.io
index` — because `--workspace` has no notion of resuming.

So the workflow asks the registry which versions are already published, skips
those, and treats a rate limit as something to wait out. Any other error stops
it at once; the loop exists to survive a limit, not to retry a real problem
until it starts looking like one.

This matters much less after the first release. The limit is on creating new
crate *names*; publishing a new version of an existing crate has a far higher
allowance, so a later release is one pass with nothing skipped and nothing
waited for.

### The attestation, and why it is not in the crate

The evidence manifest inside `ferrosift-operations` says which gate each claim
is held to: the provenance file, the licence, the workflow that builds each
target. It says `enforced`, not `passed`, and the distinction is the whole
point — committed source cannot know whether the build reading it cleared
anything. It said `passed` once, and a working tree with a red CI run produced
a manifest asserting that every check had passed.

`build-attestation.json` is the other half. It is produced by the release run,
names the revision and the run id, and records which gates that revision
actually cleared. It travels with the release rather than inside the library,
because a file shipped inside a crate can only ever describe the policy it was
written under.

Publishing by hand is still possible and is not the documented path:

```bash
cargo publish --workspace --dry-run   # what the workflow's `verify` job does
cargo publish --workspace             # needs a token in ~/.cargo/credentials.toml
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
