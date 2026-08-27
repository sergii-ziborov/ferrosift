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

## Running

```bash
cargo +nightly fuzz run decoders -- -max_total_time=60
```

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
| `bignumber` | Base conversion round trips, and arithmetic across an exponent gap |
| `jscompat` | `parseInt`, `ToInt32`, `ToUint8`, `String(x)` — small, and shared by everything |
| `dish` | Value conversion between two steps, which every recipe does and no operation owns |
| `pattern_parse` | Recursive descent over arbitrary text |
| `pattern_evaluate` | Offsets and widths computed from the source, against a buffer |

Four of them assert a property rather than only looking for a panic. `decimal`
checks that the length a value predicts is the length it renders, because the
executor refuses an oversized output on the strength of that prediction.
`bignumber` checks that a value written in a base reads back as itself.
`jscompat` checks that the two coercions agree where they must. `dish` runs two
operations rather than one, because the conversion between them belongs to
neither.

## What to do with a finding

Fix it, then pin it — as a differential case where the reference can answer, and
as a conformance assertion where it cannot. A crash that is only in
`fuzz/artifacts` is a crash that comes back.
