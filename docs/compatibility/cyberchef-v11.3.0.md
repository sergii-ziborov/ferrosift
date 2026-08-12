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
| `HMAC` | bytes | UTF-8 text | toggleString key, hash function |
| `Gzip` | bytes | bytes | compression type, filename, comment, header CRC |
| `Zlib Deflate` | bytes | bytes | compression type |
| `Zlib Inflate` | bytes | bytes | start index (+ ignored buffer knobs) |
| `To HTML Entity` | text | UTF-8 text | convert-all, named/numeric/hex |
| `From HTML Entity` | text | UTF-8 text | none |
| `ROT13` | bytes | bytes | lower/upper/numbers, amount |
| `To Charcode` | text | UTF-8 text | delimiter, base |
| `From Charcode` | text | bytes | delimiter, base |
| `Extract IP addresses` | text | UTF-8 text | IPv4/IPv6, local filter, total/sort/unique |
| `Extract URLs` | text | UTF-8 text | total/sort/unique |
| `Extract domains` | text | UTF-8 text | total/sort/unique, underscore labels |
| `Extract email addresses` | text | UTF-8 text | total/sort/unique |
| `Strings` | text | UTF-8 text | encoding, min length, match class, total/sort/unique |
| `Defang IP Addresses` | text | UTF-8 text | none |
| `Defang URL` | text | UTF-8 text | dots/http/slashes, process mode |
| `Fang URL` | text | UTF-8 text | restore dots/hxxp/slashes |
| `AES Encrypt` | text/bytes | text/bytes | key, IV, mode, I/O formats, AAD, include IV |
| `AES Decrypt` | text/bytes | text/bytes | key, IV, mode, tag, AAD, IV-from-input |
| `RC4` | text/bytes | text/bytes | passphrase, I/O formats |
| `XOR Brute Force` | bytes | UTF-8 text | key length, sample, scheme, crib |

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
- `XOR` reproduces Standard, Input/Output differential, Cascade, and null
  preserving schemes against the same key encodings as the reference.
- `Find / Replace` applies `binaryString` escape parsing to the replacement
  text; Simple and Extended modes escape the pattern before matching.
  Regex mode uses a Rust automata engine and may diverge from XRegExp for
  exotic Unicode property classes.
- Hash digests (`MD5`, `SHA1`, `SHA2`, `HMAC`) emit lower-case hex. Reduced
  SHA round counts are rejected with stable `hash.unsupported_rounds` codes;
  only the full CyberChef defaults are implemented.
- `Gzip` / `Zlib Deflate` produce interoperable streams that inflate correctly;
  compressed bytes need not match zlibjs bit-for-bit (mtime/OS/strategy).
- Named HTML entities cover a common subset; numeric and hex entities are
  complete for valid code points.
- Extractors join matches with newlines and optionally prefix
  `Total found: N`. Domain/email patterns are ASCII-centric portable ports of
  the CyberChef regular expressions; IPv6 extraction uses a conservative
  scanner rather than the full browser regex.
- `Defang URL` / `Fang URL` follow the same substitution order as CyberChef
  (`http`→`hxxp`, `.`→`[.]`, `://`→`[://]` and the reverse).
- AES supports CBC/ECB (PKCS#7 and NoPadding) and GCM. CFB/OFB/CTR are rejected
  with `crypto.aes.invalid_mode`. Empty IV becomes 16 null bytes. GCM Hex
  output appends `\n\nTag: <hex>` like the reference.
- RC4 and AES carry `legacy` / `unsafe` classifications where appropriate.
- `XOR Brute Force` enumerates keys `1..256^n-1` (never the zero key), matching
  the reference; key length is limited to 1..=2.

Where the reference produces values outside the byte range (which its node
API also rejects), decoding fails with a stable `encoding.*` /
`compression.*` / `logic.*` / `text.*` code instead: invalid characters
without filtering, Base45 triplets above 65535, binary, decimal, or octal
tokens outside `0..=255`, alphabets of the wrong size, and corrupt gzip
streams. These stable rejections are the only intentional divergences, and
none of them occur for inputs CyberChef 11.3.0 processes successfully.

## Format boundaries

This JSON format does not accept CyberChef URL/deep-link recipes or the
human-readable Chef format. Operations without an exact registered CyberChef
11.3 alias are preserved as source findings and are not executable.
