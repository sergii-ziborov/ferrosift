# Fuzz targets

Ten targets over the surfaces that read input nobody chose. The catalog is
pinned against a reference over the cases somebody thought of; this is for the
ones nobody did.

It found one on the first run: `From Charcode` split an undelimited input into
pairs by *byte* index over a `&str`, where the reference splits by UTF-16 code
unit. For ASCII the two agree, for anything else the split lands in the wrong
places, and where a pair fell inside a character it aborted the process rather
than answering. Both halves of that are now pinned by reference bytes in
`tools/cyberchef-oracle/suite/text.mjs`.

It found four more the first time `bignumber` was given both operands and a
seed corpus of exponent extremes — all of the same shape, and all of them
reachable from a recipe of about twenty characters:

| What | Cost before | After |
|---|---:|---:|
| `Divide` over a five-million-place scale gap | 34s | 0.01s |
| `Standard Deviation`'s root, which no floor covered | 29s | 0.95s |
| `x + 0`, rescaling `1e5000000` to add nothing to it | 9.1s | 0.03s |
| `0 / 1e-10000000`, a power of ten built to multiply by zero | 44s | 0.08s |

Each ended in the right verdict — the output ceiling refused every one of
them — after doing the work the verdict says should not have happened. The
fixes are three floors (`quotient_min_len`, `root_min_len`, and the existing
`sum_min_len`) and three short-circuits, and the answers are pinned against
`bignumber.js` itself in `tests/fixtures/bignumber.json`: twenty-four new pairs
either side of the rounding boundary, where a clamp that cut one place too
early would turn `5e-21` into zero.

The target's own throughput is the other measure. Before the fixes it managed
22 executions per second and ran out of memory; after them, 1,549.

## Running

```bash
cargo +nightly fuzz run decoders -- -max_total_time=60
```

Where a target has seeds, hand them to it as a second corpus directory. The
first is libFuzzer's own and is written to; the second is read-only input:

```bash
cargo +nightly fuzz run bignumber corpus/bignumber seeds/bignumber -- -max_total_time=60
```

## Seeds are not a corpus

`corpus/` is gitignored and `seeds/` is committed, and the difference is what
each one is. A corpus is a cache of whatever reached new coverage: it is
machine-generated, grows without bound, and says nothing a reader could check.
A seed is a *claim about where the interesting inputs are*, written by hand and
readable.

`seeds/bignumber` is the case for that. The arithmetic's cost is set by the gap
between two exponents, and the exponent range is ±10,000,000 — so the inputs
that matter are `1e+9999999` against `1e-9999999` and their neighbours. A
fuzzer mutating bytes will reach `1e+9999999` eventually and the pair almost
never, because it has to find both ends of a fourteen-character coincidence at
once. Twenty-four files say where to start looking.

The crate is outside the workspace on purpose: the targets link libFuzzer and
build only under nightly with sanitizer instrumentation, which every other gate
would otherwise have to carry.

On Windows the sanitizer runtime is not on `PATH` by default, and a target that
cannot find it exits `0xc0000135` with no message:

```powershell
$env:PATH = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\<version>\bin\Hostx64\x64;$env:PATH"
cargo +nightly-x86_64-pc-windows-msvc fuzz run decoders -- -max_total_time=60
```

## The targets

| Target | Surface |
|---|---|
| `decoders` | Every decoder that reads arbitrary text and claims to know what it means |
| `framing` | Framings whose next read is decided by a field they just read |
| `inflate` | The decompressors, and the budget that stops a bomb |
| `togglestring` | A key field, both readings, every option name and one that is not |
| `decimal` | `rendered_len` against `to_fixed`, which the executor's budget believes |
| `bignumber` | Base conversion round trips, and arithmetic over two parsed operands |
| `jscompat` | `parseInt`, `ToInt32`, `ToUint8`, `String(x)` — small, and shared by everything |
| `dish` | Value conversion between two steps, which every recipe does and no operation owns |
| `pattern_parse` | Recursive descent over arbitrary text |
| `pattern_evaluate` | Offsets and widths computed from the source, against a buffer |

Four of them assert a property rather than only looking for a panic. `decimal`
checks that the length a value predicts is the length it renders, because the
executor refuses an oversized output on the strength of that prediction.
`bignumber` checks that a value written in a base reads back as itself, that
addition and multiplication do not depend on operand order, and that negating
twice returns the value. `jscompat` checks that the two coercions agree where
they must. `dish` runs two operations rather than one, because the conversion
between them belongs to neither.

An identity is only worth asserting where it holds for *every* input the model
can hold, or the fuzzer spends its run rediscovering the exception. Not a
number equals nothing, itself included, so the comparisons above decline to
answer rather than claiming a failure — which is a different thing from
passing.

## What to do with a finding

Fix it, then pin it — as a differential case where the reference can answer, and
as a conformance assertion where it cannot. A crash that is only in
`fuzz/artifacts` is a crash that comes back.
