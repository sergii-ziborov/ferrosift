# Reference profiles

FerroSift's compatibility claim is against a *version*, not against a project.
"Compatible with CyberChef" is not a statement anyone can check; "produces
CyberChef 11.3.0's exact output bytes for these 2544 recipes" is.

Two versions are currently replayed:

| Profile | Commit | How it is stored |
|---|---|---|
| 11.3.0 | `d24ba1afce2e3a080308b5df7db033332fe94a1a` | baseline, in full |
| 11.4.0 | `49d1a5634a67a3b806c6db0fdca7dcecb41a776c` | delta against the baseline |

Adding a newer profile never retires an older one. A caller pinned to 11.3 is
entitled to know FerroSift still matches it, so `tests/corpus.rs` keeps
replaying 11.3 and `tests/profiles.rs` replays 11.4 alongside it.

## Why 11.4 is stored as a delta

11.4 changed nothing this corpus can see. All 2544 corpus cases and all 65
differential cases produce byte-identical output under both references.

Committing that as a second `corpus.json` would have added a megabyte of bytes
identical to the megabyte already there, and a third profile would add another.
So a non-baseline profile is stored as an overlay recording only what differs:

```json
{
  "reference": {"name": "CyberChef", "version": "11.4.0", "commit": "49d1a56…"},
  "baseline":  {"version": "11.3.0", "commit": "d24ba1a…"},
  "compared_cases": 2544,
  "changed": [], "added": [], "removed": []
}
```

This is a storage decision, not an evidential one, and the difference matters.
The test does not assert "11.4 equals 11.3, therefore FerroSift matches 11.4".
It reconstructs 11.4's own recorded case list from baseline plus delta and
replays FerroSift against *that*, case by case, at every recipe prefix. Where
the two references agree, the reconstructed byte string is 11.3's — because
11.4 produced exactly it, which the oracle checked case by case at generation
time. When upstream does change an operation, the changed case moves into
`changed` carrying 11.4's bytes, and the replay checks FerroSift against those.

`compared_cases` is what keeps the arrangement honest in both directions: it
records how much agreement the empty lists stand for, and reconstruction
asserts against it, so an overlay that silently lost cases fails rather than
passing with a smaller corpus.

## What the aliases mean

Each operation carries one `CompatibilityAlias` per profile it is proven
against. Emitting an 11.4 alias is a claim, so `tests/profiles.rs` refuses one
that no replayed 11.4 case backs — the same gate `corpus.rs` applies to 11.3.

Two failure modes are covered separately, because they fail differently:

- **Behaviour changed.** Caught by the replay: the reconstructed case carries
  11.4's bytes and FerroSift must match them.
- **The operation was renamed.** Would change no output byte, so the replay
  would pass while every recipe using the old name had quietly stopped working.
  What rules this out is that the oracle baked the 11.4 corpus *through* 11.4
  using these exact names — a rename would have failed the bake and dropped the
  case, so a name's presence in the 11.4 corpus is the evidence 11.4 still
  answers to it.

An operation whose semantics genuinely diverged between references would not be
one spec with two aliases. It would be two specs with versioned identifiers,
`…@1` and `…@2`, so a caller can ask for the behaviour they mean.

## Operations that arrive between profiles

Not every operation has existed in every profile. 11.3 exposes 501 operations
and 11.4 exposes 504, and the three it added are names 11.3 has never answered
to. A spec that claimed them in both would be claiming one of them falsely.

So a spec says which version its name *starts* existing in:

```rust
// Present in every profile, which is almost everything.
build(SpecDefinition { cyberchef_alias: Some("To Base64"), .. })

// Introduced upstream in 11.4, so aliased there and in everything after.
build_since(
    CompatibilityProfile::CyberChefV11_4,
    SpecDefinition { cyberchef_alias: Some("Modular Exponentiation"), .. },
)
```

Narrowing the range narrows what has to be proven; it exempts nothing. The
alias that *is* claimed still needs a replayed case of that profile behind it,
from the same gate every other alias answers to.

`Modular Exponentiation` is the first one this is real for rather than
illustrative. Its 76 corpus cases bake through 11.4 and fail to bake through
11.3 — the baseline reference answers `Couldn't find an operation with name
'Modular Exponentiation'` — so they arrive in the overlay's `added` list, and
the alias is backed by cases only 11.4 could produce. Nothing had to be
asserted about which version has it: the two references were asked.

The counting follows the same rule. `docs/compatibility/not-implemented.md`
partitions 11.3.0's 501 operations and does not count this one, because it is
not one of them; the ledger marks it `(since 11.4.0)`; and
`cargo xtask cyberchef gap --profile 11.4.0` reports the newer catalog's own
missing set separately.

The reverse — a name in 11.3 and not in 11.4 — would mean upstream removed an
operation. That is a different claim needing its own evidence, so
`tests/profiles.rs` refuses it rather than reading it as an oversight.

Both directions are checked against the reference itself rather than against
this description:

```bash
cargo xtask cyberchef gap --profile 11.3.0 --check
cargo xtask cyberchef gap --profile 11.4.0 --check
```

Each fails if the catalog claims a name that version of the reference does not
have. `cargo test` cannot see this: the replay gates demand evidence for the
aliases a spec carries, and a name the reference never had has no case to
demand. Only the pinned checkout can answer it, which is why the check lives
beside the oracle and not in CI.

## Reading and writing recipes

A profile is not only a naming scheme for the catalog; it is also what you read
and write a serialized recipe *as*:

```bash
ferrosift run --format cyberchef-v11.3 --recipe recipe.json --input - --input-kind bytes
ferrosift run --format cyberchef-v11.4 --recipe recipe.json --input - --input-kind bytes
```

One parser serves both, because there is nothing version-specific to parse. In
the two pinned checkouts, `src/core/Recipe.mjs`, `Operation.mjs`, `Dish.mjs`
and `Utils.mjs` are byte-identical; the reference's recipe model did not change
between 11.3 and 11.4. What the flag selects is which operation *names*
resolve, and that is the whole difference:

| | 11.3 | 11.4 |
|---|---|---|
| `[{"op":"To Base64","args":[…]}]` | loads | loads |
| `[{"op":"Modular Exponentiation","args":[…]}]` | `compat.cyberchef.unknown_operation` | loads |

The finding names the version that was asked, because the version is what
decides the answer. "No exact CyberChef 11.3 alias" is a fact about 11.3, and a
message that omitted it would read as FerroSift not having the operation.

Export is the same asymmetry from the other side. `export_recipe` writes each
step under the name the requested profile uses, and refuses with
`ExportError::MissingAlias` when that profile has no name for it — emitting the
11.4 name into a file labelled 11.3 would produce a recipe the older reference
cannot load. `CompatibilityProfile::Native` is a catalog naming profile rather
than a serialized dialect, so both directions reject it outright with
`UnsupportedProfile` instead of half-working.

`crates/ferrosift-compat/tests/profile_scope.rs` holds all of this against a
synthetic operation that exists in exactly one profile. A real 11.4-only
operation would let those tests pass for a second reason — that this port
happens to have it — and they would stop failing if the profile argument were
ignored.

### The one core change 11.4 did make

`Ingredient.mjs` is the single core module that differs, by 24 lines: 11.4's
`validate()` gained an `argSelector` branch that refuses a value not among the
declared option names. That tightens what the reference accepts as an
*argument*, not what it accepts as a *recipe*, so it does not reach the parser.

FerroSift refuses such a value in both profiles, at the operation rather than
at the ingredient — `SHA2` with an unrecognised size answers
`hash.sha2.invalid_size` under `cyberchef-v11.3` as well as under
`cyberchef-v11.4`. Nothing in the corpus depends on 11.3's laxer reading,
because a case whose argument the reference rejects produces no output bytes to
pin.

## Adding a profile

```bash
cargo xtask cyberchef setup    --profile 11.5.0
cargo xtask cyberchef generate --profile 11.5.0
cargo xtask cyberchef overlay  --profile 11.5.0
```

`setup` clones at the pinned commit, runs `npm ci`, and then `npx grunt node` —
the reference's node entry point `src/node/index.mjs` is generated by its own
build rather than committed, so a fresh clone has no entry point until that
runs.

`generate` writes the full fixtures; `overlay` condenses them against the
baseline. Only the overlay is committed — the full files are gitignored.
