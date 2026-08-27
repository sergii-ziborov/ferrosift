# CyberChef 11.3.0 JSON interchange

FerroSift imports and exports JSON recipe arrays in the CyberChef 11.3.0
browser save format when every executable operation has an exact registered
`CyberChefV11_3` alias.

## Recipe format

The top-level value is an array. Each supported step contains:

- `op`: the exact CyberChef operation name;
- `args`: positional JSON arguments;
- `disabled`: an optional boolean, defaulting to `false`;
- `breakpoint`: an optional boolean, defaulting to `false`.

## Limits

- Maximum serialized input or output: 1 MiB.
- Maximum recipe length: 4096 steps.
- Maximum executable list/map argument depth: 120 containers.
- Executable integers: JavaScript safe-integer range,
  `-9007199254740991..=9007199254740991`.

The execution budget is a FerroSift concept the reference does not have, and
one place it now applies *earlier* than the executor alone would. Exact
addition brings both operands to the finer of the two exponents, so the gap
between them is the answer's width: `1e10000000 + 1e-10000000` is twenty-three
characters of input and twenty million digits of answer. The executor refused
that already — after five seconds spent building the digits it then discarded.
The list-arithmetic operations now compare a floor on the answer's width
against the budget before taking each step, and stop at the one that would
cross it.

That is narrower than the executor in one case, deliberately: `1e10000000 +
1e100 - 1e10000000` has a short answer and a twenty-million-digit middle, and
it is now refused. An intermediate nobody can hold is the resource the budget
exists to bound, and a recipe that genuinely wants one can raise
`max_output_bytes`.

## Import behavior

Operation names use exact, case-sensitive `CyberChefV11_3` aliases. No fuzzy
matching, normalization, or fallback runtime is used.

The import result preserves the complete semantic source JSON within the
limits. Unknown operations or fields, invalid flags, missing fields, extra
arguments, incompatible types, unsafe integers, and excessive argument depth
produce ordered findings with stable codes and source-step positions.

An executable FerroSift recipe is available only when every step maps without
an error finding. Source re-export preserves JSON values, fields, numeric
tokens, and step order. Whitespace and object-key order are not byte-for-byte
contracts.

## Export behavior

Export requires every operation to be registered with exactly one CyberChef
11.3 alias. Named arguments are emitted in their declared positional order.
Missing required arguments, undeclared arguments, ambiguous aliases, unsafe
integers, excessive nesting, and output beyond the size limit return explicit
errors.

`disabled` and `breakpoint` are emitted only when true.

## Registered operations

The built-in registry provides these exact CyberChef 11.3 aliases:

| Alias | Input | Output | Arguments |
|---|---|---|---|
| `To Hex` | bytes | UTF-8 text | delimiter, bytes per line |
| `From Hex` | text | bytes | delimiter or automatic detection |
| `To Base32` | bytes | UTF-8 text | alphabet expression |
| `From Base32` | text | bytes | alphabet, filtering |
| `To Base45` | bytes | UTF-8 text | alphabet expression |
| `From Base45` | text | bytes | alphabet, filtering |
| `To Base58` | bytes | UTF-8 text | alphabet expression |
| `From Base58` | text | bytes | alphabet, filtering |
| `To Base64` | bytes | UTF-8 text | alphabet expression |
| `From Base64` | text | bytes | alphabet, filtering, strict mode |
| `To Base85` | bytes | UTF-8 text | alphabet, delimiter wrap |
| `From Base85` | text | bytes | alphabet, filtering, zero-group character |
| `To Binary` | bytes | UTF-8 text | delimiter, byte length |
| `From Binary` | text | bytes | delimiter, byte length |
| `To Decimal` | bytes | UTF-8 text | delimiter, signed values |
| `From Decimal` | text | bytes | delimiter or Auto, signed values |
| `To Octal` | bytes | UTF-8 text | delimiter |
| `From Octal` | text | bytes | delimiter |
| `URL Encode` | bytes | UTF-8 text | encode all special characters |
| `URL Decode` | text | UTF-8 text | treat `+` as space |
| `To Hexdump` | bytes | UTF-8 text | width, case, final length, UNIX format |
| `From Hexdump` | text | bytes | none |
| `XOR` | bytes | bytes | toggleString key, scheme, null preserving |
| `Gunzip` | bytes | bytes | none |
| `Take bytes` | bytes | bytes | start, length, per-line |
| `Drop bytes` | bytes | bytes | start, length, per-line |
| `Head` | text | UTF-8 text | delimiter token, count |
| `Find / Replace` | text | UTF-8 text | toggleString find, replace, flags |
| `MD5` | bytes | UTF-8 text | none |
| `SHA1` | bytes | UTF-8 text | rounds (full 80 only) |
| `SHA2` | bytes | UTF-8 text | size, rounds (full defaults only) |
| `SHA3` | bytes | UTF-8 text | size 224/256/384/512 |
| `HMAC` | bytes | UTF-8 text | toggleString key, hash function |
| `Gzip` | bytes | bytes | compression type, filename, comment, header CRC |
| `Zlib Deflate` | bytes | bytes | compression type |
| `Zlib Inflate` | bytes | bytes | start index (+ ignored buffer knobs) |
| `Raw Deflate` | bytes | bytes | compression type |
| `Raw Inflate` | bytes | bytes | start index (+ ignored buffer knobs) |
| `Bzip2 Compress` | bytes | bytes | block size 1-9, work factor (ignored) |
| `Bzip2 Decompress` | bytes | bytes | low-memory flag (ignored) |
| `To HTML Entity` | text | UTF-8 text | convert-all, named/numeric/hex |
| `From HTML Entity` | text | UTF-8 text | none |
| `ROT13` | bytes | bytes | lower/upper/numbers, amount |
| `To Charcode` | text | UTF-8 text | delimiter, base |
| `From Charcode` | text | bytes | delimiter, base |
| `Extract IP addresses` | text | UTF-8 text | IPv4/IPv6, local filter, total/sort/unique |
| `Extract URLs` | text | UTF-8 text | total/sort/unique |
| `Extract domains` | text | UTF-8 text | total/sort/unique, underscore labels |
| `Extract email addresses` | text | UTF-8 text | total/sort/unique |
| `Extract MAC addresses` | text | UTF-8 text | total/sort/unique (hex sort, exact unique) |
| `Extract hashes` | text | UTF-8 text | char length or all common sizes, total |
| `Extract file paths` | text | UTF-8 text | Windows/UNIX, total/sort/unique |
| `Strings` | text | UTF-8 text | encoding, min length, match class, total/sort/unique |
| *(native)* `Suggest recipe` | text/bytes | UTF-8 text | depth, max results, intensive, crib |
| `Fork` | text/bytes | UTF-8 text | split delimiter, merge delimiter, ignore errors |
| `Merge` | any | any | merge_all |
| `Defang IP Addresses` | text | UTF-8 text | none |
| `Defang URL` | text | UTF-8 text | dots/http/slashes, process mode |
| `Fang URL` | text | UTF-8 text | restore dots/hxxp/slashes |
| `AES Encrypt` | text/bytes | text/bytes | key, IV, mode, I/O formats, AAD, include IV |
| `AES Decrypt` | text/bytes | text/bytes | key, IV, mode, tag, AAD, IV-from-input |
| `AES Key Wrap` | text/bytes | text/bytes | KEK, 8-byte IV, I/O formats |
| `AES Key Unwrap` | text/bytes | text/bytes | KEK, 8-byte IV, I/O formats |
| `Derive PBKDF2 key` | ignored | UTF-8 hex text | passphrase, key size bits, iterations, hash, salt |
| `Scrypt` | text/bytes | UTF-8 hex text | salt, N, r, p, key length |
| `RC4` | text/bytes | text/bytes | passphrase, I/O formats |
| `XOR Brute Force` | bytes | UTF-8 text | key length, sample, scheme, crib |

## Verified corpus

Compatibility is measured, not asserted. Two machine-generated fixtures pin
FerroSift against the reference at commit
`d24ba1afce2e3a080308b5df7db033332fe94a1a`:

- a curated differential suite of **65** representative recipes, and
- an automatic corpus of **518** deterministically sampled cases.

Every case is replayed through the real executor and must match the reference
output bytes and stopping position at **every recipe prefix** (**583** pinned
cases total). A coverage gate fails the build if any CyberChef-aliased
operation has no corpus case and no documented exemption, so no operation is
silently unverified.

Both fixtures are reproducible by anyone. The generator lives in
[`tools/cyberchef-oracle`](../../tools/cyberchef-oracle/README.md) and is
driven by `cargo xtask cyberchef generate`; it refuses to run against any
commit other than the pin, and re-running it produces byte-identical output.
`cargo xtask cyberchef gap` reports which reference operations are still
unimplemented, derived from the reference catalog and the live FerroSift
catalog rather than from a hand-maintained list. The only exemptions are the compressor directions (whose
output is interoperable, not bit-identical; their inverse is byte-pinned) and
the `Fork` / `Merge` flow-control pair (pinned by dedicated tests).

## Conformance profile

Outputs are byte-for-byte identical to CyberChef 11.3.0 for every input the
reference processes into valid bytes, including its observable quirks:

- Base32 partial trailing groups reproduce the reference bit arithmetic,
  including the extra byte a lone ninth symbol produces.
- Base45 pads short groups with a literal `0` character regardless of the
  alphabet, masks two-symbol groups to their low byte, and accepts lone
  trailing symbols.
- Base58 counts leading zero symbols before non-alphabet characters are
  removed, so noise ahead of them drops the zero bytes.
- Base85 compresses any all-zero block in the standard alphabet to `z`
  (including partial trailing blocks, which do not round-trip), wraps block
  arithmetic modulo 2^32, ignores one dangling symbol, and keeps the raw `-1`
  digit of an embedded zero-group symbol.
- `From Binary`'s `Space` and `None` delimiters strip every JavaScript
  whitespace character; token parsing follows JavaScript `parseInt`, so
  unparsable tokens coerce to zero bytes wherever the reference emits them.
- `From Octal` splits on the literal delimiter and keeps empty tokens as zero
  bytes; `From Decimal Auto` splits on non-digit runs while keeping dashes.
- `URL Decode` never fails: strict UTF-8 percent decoding falls back to the
  legacy `unescape` algorithm (`%XX` and `%uXXXX` code units), matching the
  reference's error handling exactly.
- `To Hexdump` pads the hex field to `width * 3` columns, keeps Latin-1 bytes
  in the ASCII gutter, and — with the final-length line enabled — emits that
  trailer as raw **lower-case** hex even in upper-case mode, exactly as the
  reference pushes it after per-line upper-casing.
- `XOR` reproduces Standard, Input/Output differential, Cascade, and null
  preserving schemes against the same key encodings as the reference.
- `Find / Replace` applies `binaryString` escape parsing to the replacement
  text; Simple and Extended modes escape the pattern before matching.
  Regex mode uses a Rust automata engine and may diverge from XRegExp for
  exotic Unicode property classes.
- Hash digests (`MD5`, `SHA1`, `SHA2`, `SHA3`, `HMAC`) emit lower-case hex. Reduced
  SHA round counts are rejected with stable `hash.unsupported_rounds` codes;
  only the full CyberChef defaults are implemented.
- `Gzip` / `Zlib Deflate` / `Raw Deflate` produce interoperable streams that
  inflate correctly; compressed bytes need not match zlibjs bit-for-bit
  (mtime/OS/strategy).
- `Raw Inflate` accepts a start index like `Zlib Inflate` and ignores CyberChef
  buffer knobs.
- `Bzip2 Compress` / `Bzip2 Decompress` interoperate with libbzip2 / CyberChef
  streams. Compressed bytes need not match the reference encoder bit-for-bit;
  empty input is rejected with `compression.bzip2.empty_input`. The work-factor
  and low-memory arguments are accepted for interchange and ignored.
- Named HTML entities cover a common subset; numeric and hex entities are
  complete for valid code points.
- Extractors join matches with newlines and optionally prefix
  `Total found: N` (`Extract hashes` uses `Total Results: N`). Domain/email
  patterns are ASCII-centric portable ports of the CyberChef regular
  expressions; IPv6 extraction uses a conservative scanner rather than the full
  browser regex. MAC extraction uses colon/dash forms with hexadecimal sort and
  exact (case-sensitive) unique. Hash extraction matches **lowercase** hex only,
  like the reference. `Extract domains` reproduces the reference match set
  (including its quirk of reading `cmd.exe` as a domain); because the portable
  automata engine has no lookaround, the reference's `{1,63}` per-label cap is
  dropped, so domain labels **longer than 63 characters** are a documented
  micro-divergence that does not arise for real domains.
- `Defang URL` / `Fang URL` follow the same substitution order as CyberChef
  (`http`→`hxxp`, `.`→`[.]`, `://`→`[://]` and the reverse).
- AES supports CBC/CFB/OFB/CTR/ECB (PKCS#7 and NoPadding for CBC/ECB) and GCM.
  Empty IV becomes 16 null bytes. GCM Hex output appends `\n\nTag: <hex>` like
  the reference. Stream modes use 128-bit feedback / big-endian CTR.
- `AES Key Wrap` / `AES Key Unwrap` implement RFC 3394 with a configurable
  8-byte IV (default `a6a6a6a6a6a6a6a6`).
- `Derive PBKDF2 key` is deterministic only: empty salt is rejected with
  `crypto.pbkdf2.empty_salt` (CyberChef would generate a random salt). Key size
  is in bits and must be a positive multiple of 8.
- `Scrypt` accepts empty salt (deterministic). `N` must be a power of two ≥ 2.
- RC4 and AES carry `legacy` / `unsafe` classifications where appropriate.
- `XOR Brute Force` enumerates keys `1..256^n-1` (never the zero key), matching
  the reference; key length is limited to 1..=2. The text sample is UTF-8
  decoded (Latin-1 fallback) and its control bytes `0x09..=0x10` are shifted
  into the `U+E000` private-use area (`Utils.escapeWhitespace`) so a decoded
  newline cannot split a record; the hex-output mode emits spaced lower-case
  hex.

Where the reference produces values outside the byte range (which its node
API also rejects), decoding fails with a stable `encoding.*` /
`compression.*` / `logic.*` / `text.*` code instead: invalid characters
without filtering, Base45 triplets above 65535, binary, decimal, or octal
tokens outside `0..=255`, alphabets of the wrong size, and corrupt gzip
streams. These stable rejections are the only intentional divergences, and
none of them occur for inputs CyberChef 11.3.0 processes successfully.

## Where this page and the ledger meet

Every divergence argued below is also recorded in
[`divergences.json`](divergences.json), one entry per operation, naming the
domain it applies to and the section here that argues it. The ledger reads that
file to separate two questions it used to answer with one word:

- **Evidence** — how an operation was checked. 237 are differential-pinned
  against the reference, 4 by a named test the automatic corpus cannot reach,
  4 through a pinned inverse, and none by nothing at all.
- **Parity** — how close it came. 224 match the reference everywhere this
  project knows of, 17 diverge over a stated domain, 4 are interoperable rather
  than byte-identical, and 2 are FerroSift's own with no reference to match.

The two are not the same claim and an operation can have the first without the
second. A corpus covers the cases it holds; it cannot speak for the ones it
does not, and `exact` was being read as though it could. Every operation listed
as diverging is byte-pinned over its own corpus — the divergence is what lies
outside it, which is exactly what a case count is unable to say.

The generator refuses an entry whose alias is not registered or whose section
is not on this page, so neither half can quietly outlive the other.

## Format boundaries

### Delimiter arguments are values, not spellings

CyberChef's `binaryString` arguments carry an escaped spelling in its
interface — `\n\n` written as a backslash, an `n`, a backslash, an `n` — which
`Ingredient.prepare` unescapes before the operation runs. Its recipe API does
not unescape: an operation baked through `chef.bake` receives whatever string
it was given.

FerroSift's arguments are typed, and a `Text` argument carries a value rather
than a spelling of one. `\n` in a FerroSift recipe is a newline. This matches
the reference's API path exactly, which is what the corpus pins; a recipe
copied out of the CyberChef interface will carry the escaped form and must be
unescaped by whatever produces it, not by the operation.

This was found by baking, not by reading: every set and distance case with an
escaped delimiter failed against the reference until the delimiters were
passed literally.

### Power Set is not implemented

`Power Set` is registered by the reference but does not compute one. For every
input tested it returns exactly two lines — the empty subset and the full set
— because its subset enumeration builds a list of numbers and then calls a
string method on each of them.

There are two ways to be compatible with that and neither is worth shipping.
Reproducing it exactly would put an operation in the catalog that does not do
what its name says, under a claim of verified compatibility. Computing a real
power set would be a silent divergence from the reference in an operation
claiming to match it.

So it is absent, and this note is the record of why. If the reference fixes
it, FerroSift can implement the fixed behaviour and pin it like everything
else.

### An omitted argument is its declared default, not `undefined`

A recipe saved by the reference carries every argument, so this is only
reachable in hand-written JSON. Where it is reachable, the two disagree: the
reference reads `args[]` positionally and a missing entry is `undefined`, which
for a boolean is falsy. `{"op": "URL Decode", "args": []}` therefore runs with
`Treat "+" as space` **off** there, despite the operation declaring that
argument's default as **on**.

FerroSift applies the declared default instead, so the same JSON runs with it
on. Neither reading is obviously right — the reference disagrees with its own
declared default, and matching that would mean an omitted argument meaning
something different from the value the catalog publishes for it.

The safe form is to write every argument explicitly, which is what the
reference's own export does.

### From Hex refuses an odd number of digits

The reference reads a trailing lone hex digit as a byte with a zero high
nibble, so `abc` decodes to `ab 0c` and `not-hex` decodes to nothing at all.
FerroSift refuses both with `encoding.hex.odd_length`.

| Input | Reference | FerroSift |
|---|---|---|
| `abc` | `ab 0c` | refuses |
| `a` | `0a` | refuses |
| `68 69 6` | `68 69 06` | refuses |
| `not-hex` | *(empty)* | refuses |

The reason is the last row. Automatic delimiter detection means any text can be
offered to this operation, and text with no hex digits in it decodes to an
empty result rather than an error — so a mistyped recipe or a wrong input
produces silence instead of a complaint. Refusing turns that into a message,
at the cost of also refusing the three rows above it, which are unambiguous.

That trade is arguable in both directions and this note exists so it is
arguable rather than invisible. `tests/safety.rs` pins the refusal.

### Object identifiers refuse where the reference answers `NaN`

`Object Identifier to Hex` and `Hex to Object Identifier` both hand text to a
bignum whose parser skips characters it does not recognise. Where the text
contains such characters, the reference returns a number derived from the
letters of the word `NaN`:

| Operation | Input | Reference | FerroSift |
|---|---|---|---|
| Object Identifier to Hex | `1` | `NaN` | refuses |
| Object Identifier to Hex | `1..2` | `NaN02` | refuses |
| Hex to Object Identifier | *(empty)* | `NaN.NaN` | refuses |
| Hex to Object Identifier | `2azz` | `1.2.95` | refuses |

`95` is what you get from reading `N`, `a`, `N` as bignum digits. This is a
divergence, not compatibility, and it is recorded as one here rather than left
to be discovered.

Two reasons for refusing. No caller wants `95` for input `zz`, and an operation
that answers a question it cannot answer is more dangerous than one that says
so. And reproducing it would mean reproducing a specific bignum's digit table
and word-size carry behaviour — pinned to that library rather than to any
specification, and a large amount of machinery for output nobody should act on.

Everything reachable from well-formed input matches byte for byte. That
includes two bugs, which *are* reproduced because well-formed input reaches
them:

- A first arc pair above 255 is written as plain hexadecimal with no base-128
  continuation and no padding, so `2.999` produces `437` — three hex digits
  that no ASN.1 decoder, including FerroSift's own, reads back as `2.999`.
- The first pair is computed with JavaScript doubles while every later arc goes
  through an exact big integer, so `9007199254740993.1` and
  `9007199254740992.1` encode identically while `1.2.9007199254740993` and
  `1.2.9007199254740992` do not.

Both are pinned in the corpus. `tests/conformance_framing.rs` holds the
refusals and states the rounding property.

### TEA and XTEA reproduce two of the reference's own bugs

Both are pinned in the corpus, because well-formed input reaches them.

**`BIT` padding does not round-trip a message that already fills its blocks.**
`applyPadding` returns early for every scheme but PKCS#5 when nothing needs
adding, so the `0x80` marker is never written — and the removal then scans back
for a marker that is not there and throws. Encrypting eight bytes with `BIT`
succeeds; decrypting the result with `BIT` fails, in both projects.

**`ZERO` and `RANDOM` padding are added and never removed.** Neither leaves a
marker, so `removePadding` hands back the padded plaintext whole. A round trip
through either returns more bytes than it was given, and the extra bytes are
part of the answer rather than an error.

`RANDOM` is the one argument with no output to pin: the reference fills those
bytes with `Math.random()`. FerroSift refuses it in exactly the cases where the
reference would have been unpredictable — when padding is actually added — and
accepts it everywhere else, which is every message that already fills its
blocks. `tests/conformance_tea.rs` holds all three.

### A toggleString field is read two different ways

`Utils.convertToByteArray` and `Utils.convertToByteString` are two functions,
not one, and every toggleString field in the catalog goes through one or the
other. They agree on Hex, Binary, Decimal, Base64 and UTF8. They disagree on
**Latin1**, and on any option name neither recognises, which both treat as
Latin1:

| Field | Reads with | `Latin1` field `日本` becomes |
|---|---|---|
| XOR, AND, OR, ADD, SUB, XXTEA, BLAKE2, Scrypt salt | `convertToByteArray` | `e6 97 a5 e6 9c ac` — the string's UTF-8 |
| AES, AES Key Wrap/Unwrap, PBKDF2, HMAC | `convertToByteString` | `e5 2c` — each code unit's low byte |

The array reading is `strToByteArray`, which takes code units directly while
every one fits in a byte and switches to UTF-8 encoding the *whole* string as
soon as one does not. The string reading hands the string over untouched and
leaves the masking to whichever library receives it. Both are reproduced;
`key.rs` holds them and the `togglestring` corpus family pins the same twenty
five fields through both readings so they cannot be collapsed back into one.

Four properties of those readings are worth stating, because a stricter port
is the natural thing to write and would be wrong in each:

| Option | Field | Reference | A strict port |
|---|---|---|---|
| Hex | `abc` | `ab 0c` | refuses an odd digit |
| Hex | `0x41 0x42` | `41 42` | refuses `x` |
| Hex | `zz` | *(empty)* | refuses |
| Base64 | `!QUJD!` | `41 42 43` | refuses `!` |
| Binary | `0100 000101000010` | `41 42` | restarts at the gap |
| Decimal | `1,2,3` | `01 02 03` | splits on spaces only |

Note the third row against [From Hex](#from-hex-refuses-an-odd-number-of-digits)
above: the *operation* refuses what this *field* accepts, deliberately. The
operation is a decoder whose silence on garbage would be a bug; the field is a
key, where the reference's own reading is the only thing that reproduces the
reference's ciphertext.

**One divergence remains, in HMAC alone.** `crypto-api` packs four characters
into a thirty-two bit word with `charCodeAt(i) << 24 | charCodeAt(i+1) << 16 |
…` and no mask, so a key character above 255 spills its high byte into the
*previous* character's position. A `Latin1` key of `日本` produces a digest that
is not the HMAC of any key under any encoding — reproducing it would mean
reproducing that library's word packing rather than HMAC. FerroSift computes
the HMAC of `e5 2c`, which is what the reading says the key is.

This is reachable only through the `Latin1` option (or a misspelt one) with
text outside the byte range; every other option produces characters that fit,
where the two agree exactly. `tests/conformance_togglestring.rs` pins both the
agreement and the divergence.

### A toggleString field does not hold bytes

`fromDecimal` is `parseInt` per field and nothing else, so the array
`convertToByteArray` returns holds whatever that produced: `300` for a field of
`300`, `-1`, `NaN` for a field of `-`, and `Infinity` for a run of digits long
enough to overflow a double. `fromBinary` chunks eight characters at a time and
so cannot exceed 255, but a chunk starting on a character `parseInt` will not
read is `NaN` there too.

Only two families offer either option, because `toggleValues` decides what a
field can be:

| Field | Options | Can hold a non-byte |
|---|---|---|
| XOR, AND, OR, ADD, SUB | Hex, **Decimal**, **Binary**, Base64, UTF8, Latin1 | yes |
| BLAKE2b, BLAKE2s | UTF8, **Decimal**, Base64, Hex, Latin1 | yes |
| XXTEA, Scrypt salt, TEA, XTEA | Hex, UTF8, Latin1, Base64 | no |

What happens next is the consumer's, and it is not one rule. **BLAKE2, XXTEA
and Scrypt** store the array into a `Uint8Array` or a `Buffer`, which is
`ToUint8`: `300` is `44` and `NaN` is `0`. **The bitwise family** does not
coerce the key at all — `bitOp` applies the operator to the number and pushes
the result — and `Dish.valid()` then walks the finished array refusing any
element `< 0` or `> 255`.

So the same key succeeds or fails depending on the operator:

| Operator | Key `300` on byte `b` | Result |
|---|---|---|
| `AND` | `b & 300` | always a byte; succeeds |
| `ADD` | `(b + 300) % 256` | always a byte; succeeds |
| `SUB` | `b - 300`, corrected by one `+256` | a byte only for `b >= 44` |
| `OR` | `b \| 300` | keeps bit 8; always refused |
| `XOR` | `b ^ 300` | keeps bit 8; always refused |

The refusal is the dish's, not the operation's, so FerroSift reports it under
one code — `core.dish.invalid_byte_array` — rather than five.

Two consequences do not follow from masking the key to a byte first, which is
the natural thing to write:

- **`NaN` is out of range and still allowed.** `Dish.valid()` compares, and
  `NaN` fails both comparisons, so it reaches the output and becomes `0` when
  the array is stored. `ADD` and `SUB` are arithmetic and carry it through, so a
  key of `-` *erases* the input; `XOR`, `AND` and `OR` convert it away with
  `ToInt32` first, so the same key behaves as a zero byte. A port that masked
  `NaN` to `0` has `ADD` and `SUB` exactly backwards.
- **Null preserving compares before anything narrows.** `o === k` is equality
  between two JavaScript numbers, and `44` is not `300`: the reference XORs
  them and the dish refuses the `256` that comes out. Masking the key first
  makes them equal, preserves the byte, and returns successfully with the wrong
  answer.

Everything the reference will actually run is pinned in the `differential`
suite. The refusals are in `tests/conformance_togglestring.rs` instead, because
this corpus records reference *output* and a recipe the reference declines to
run has none — the oracle cannot bake `XOR` with a key of `300` at all.

### Flow control: Fork / Merge

`Fork` / `Merge` are first-class map/join control (not jump soup). The executor
uses a single recursive region interpreter (`execute_region`) so nested Fork
bodies are real nested regions, not plain `operation.execute` calls. Each
region understands normal ops and Fork (future conditionals/subsections share
the same entry point). Missing Merge means the body runs to the end of the
enclosing region. `ignore_errors` replaces a failing branch with an empty
string.

Flow work is bounded beyond plain output size via `ExecutionBudget`:
`max_branches`, `max_flow_depth`, `max_operation_invocations`, and
`max_total_bytes_processed`. Node CyberChef excludes Fork, so differential
fixtures do not bake Fork recipes against the pinned oracle; conformance is
native.

Note: current Fork is still CyberChef-shaped (text split/join). A future
native `flow.map` / `flow.join` over `Value::List` is the intended pure
dataflow primitive without Unicode re-encoding.

### Magic-as-advisor (native only)

CyberChef `Magic` remains an **unsupported** interchange operation (flow-control
black box). FerroSift instead ships native `analysis.suggest@1` / **Suggest
recipe**, which:

- never rewrites the input into a guessed decode result;
- ranks portable catalog ops (hex/base64/base32/url/html/fang/gzip/bzip2/zlib/raw
  inflate, optional ROT13 in intensive mode);
- emits a deterministic text report with scores, previews, and CyberChef-shaped
  `recipe: [...]` fragments for copy/paste;
- accepts `depth` (1–3), `max_results`, `intensive`, and a literal `crib`
  substring filter.

There is intentionally **no** `Magic` CyberChef alias, so imported `Magic`
recipes still fail closed with `compat.cyberchef.unknown_operation`.

This JSON format does not accept CyberChef URL/deep-link recipes or the
human-readable Chef format. Operations without an exact registered CyberChef
11.3 alias are preserved as source findings and are not executable.
