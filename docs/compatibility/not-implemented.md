# Operations not implemented

FerroSift covers 228 of CyberChef 11.3.0's 503 catalog operations. This page
records what the other 275 are waiting on, so that "not done yet" is a list
with reasons rather than a number.

The grouping below is by *import*, which is a proxy and not the thing itself.
`To Base` sat in the no-dependency list for three revisions because it never
imports `bignumber.js` — it receives a `BigNumber` from the dish and calls a
method on it. Reading the imports found nothing to port; reading the operation
found an arbitrary-precision rendering. Where a listing here and the code
disagree, the code is right.

The rule that decides everything here is the one the rest of the project runs
on: a compatibility claim must be backed by a pinned differential corpus. An
operation ships only when its bytes match the reference's bytes on every case
in [the corpus](cyberchef-v11.3.0.md).

## Why an equivalent library is not enough

168 of the 275 are built on a JavaScript library, and 21 more reach one
through an internal library of the reference's own. The three headings below
partition the 275 exactly: 168 plus 21 plus 86. The obstacle is **not** that
Rust lacks equivalents -- it usually has good ones. It is that byte-exactness
is against *that* library, not against a library that does the same job.

`comrak` renders correct HTML from Markdown, and it differs from `markdown-it`
in whitespace, attribute order, and entity choice. `syntect` highlights code,
and shares no output format with `highlight.js`. `image` decodes and re-encodes
a PNG, and does not produce `jimp`'s bytes. Every case in such a corpus would
fail, and it would fail for a reason that says nothing about either library
being wrong.

So each of these has exactly two honest routes:

1. **Port the specific library's algorithm**, faithfully enough to match it.
   That is a project per library, not a task per operation.
2. **Ship the operation without a compatibility claim**, documented as
   FerroSift's own behaviour rather than the reference's.

Shelling out to npm is not a third route. FerroSift is a Rust library that a
Node binding may wrap, not a wrapper around Node.

## Ranked by what a port would unlock

The count is how many operations one port would make reachable, which is the
only ordering that matters when choosing what to do next.

| Library | Operations | Rust starting point | Verdict |
|---|---|---|---|
| `jimp` | 23 | `image` | Pixel-exact re-encoding is the hard part, not decoding. |
| `bignumber.js` | 5 | `num-bigint`, already a dependency | **Mostly done.** The arithmetic and base conversions are built and pinned; each of the rest needs a second thing as well. |
| `node-forge` | 15 | `rsa`, `x509-cert` | PKI structure is standardised; the text rendering is not. |
| `jsrsasign` | 12 | `x509-parser` | As above. |
| `es6-promisify` | 7 | none needed | A promise shim; the operations behind it may be portable once read. |
| `vkbeautify` | 6 | hand-written | Pretty-printing rules are short and fully determined by the source. |
| `@wavesenterprise/crypto-gost-js` | 6 | `gost-crypto` crates exist | GOST is standardised; the binding is not. |
| `d3` | 5 | none | Chart layout is the library. A port is a port of d3. |
| `moment-timezone` | 5 | `chrono-tz` | The IANA database matches; the formatting grammar does not. |
| `codepage` | 4 | `encoding_rs` | Table-driven and therefore checkable. |

The full grouping follows, so that picking any one of them starts from a
list rather than a search.

| Library | Count | Operations |
|---|---|---|| `jimp` | 23 | Add Text To Image, Blur Image, Contain Image, Convert Image Format, Cover Image, Crop Image, Dither Image, Extract LSB, Extract RGBA, Flip Image, Generate Image, Image Brightness / Contrast, Image Filter, Image Hue/Saturation/Lightness, Image Opacity, Invert Image, Normalise Image, Randomize Colour Palette, Resize Image, Rotate Image, Sharpen Image, Split Colour Channels, View Bit Plane |
| `bignumber.js` | 5 | From BCD, Parse IPv6 address, Parse TCP, Pseudo-Random Number Generator, To BCD |
| `node-forge` | 15 | Blowfish Decrypt, Blowfish Encrypt, CMAC, DES Decrypt, DES Encrypt, Generate RSA Key Pair, Pseudo-Random Integer Generator, RC2 Decrypt, RC2 Encrypt, RSA Decrypt, RSA Encrypt, RSA Sign, RSA Verify, Triple DES Decrypt, Triple DES Encrypt |
| `jsrsasign` | 12 | ECDSA Sign, ECDSA Signature Conversion, ECDSA Verify, Generate ECDSA Key Pair, JWK to PEM, Parse ASN.1 hex string, Parse CSR, Parse X.509 certificate, Parse X.509 CRL, PEM to JWK, Public Key from Certificate, Public Key from Private Key |
| `es6-promisify` | 7 | Generate PGP Key Pair, PGP Decrypt, PGP Decrypt and Verify, PGP Encrypt, PGP Encrypt and Sign, PGP Sign, PGP Verify |
| `vkbeautify` | 6 | CSS Beautify, CSS Minify, JSON Minify, SQL Minify, XML Beautify, XML Minify |
| `@wavesenterprise/crypto-gost-js/index.js` | 6 | GOST Decrypt, GOST Encrypt, GOST Key Unwrap, GOST Key Wrap, GOST Sign, GOST Verify |
| `d3` | 5 | Entropy, Heatmap chart, Hex Density chart, Scatter chart, Series chart |
| `moment-timezone` | 5 | DateTime Delta, From UNIX Timestamp, Parse DateTime, To UNIX Timestamp, Translate DateTime Format |
| `codepage` | 4 | Decode text, Encode text, MIME Decoding, Text Encoding Brute Force |
| `bson` | 3 | BSON deserialise, BSON serialise, Parse ObjectID timestamp |
| `bcryptjs` | 3 | Bcrypt, Bcrypt compare, Bcrypt parse |
| `crypto-api/src/crypto-api.mjs` | 3 | Derive HKDF key, Flask Session Sign, Flask Session Verify |
| `xregexp` | 3 | Filter, Register, Regular expression |
| `jsonwebtoken` | 3 | JWT Decode, JWT Sign, JWT Verify |
| `js-ascon` | 3 | Ascon Decrypt, Ascon Encrypt, Ascon Hash |
| `rison` | 2 | Rison Decode, Rison Encode |
| `uuid` | 2 | Analyse UUID, Generate UUID |
| `notepack.io` | 2 | From MessagePack, To MessagePack |
| `blakejs` | 2 | BLAKE2b, BLAKE2s |
| `fernet` | 2 | Fernet Decrypt, Fernet Encrypt |
| `otpauth` | 2 | Generate HOTP, Generate TOTP |
| `cbor` | 2 | CBOR Decode, CBOR Encode |
| `@xmldom/xmldom` | 2 | CSS selector, XPath expression |
| `@blu3r4y/lzma` | 2 | LZMA Compress, LZMA Decompress |
| `ctph.js` | 2 | Compare CTPH hashes, CTPH |
| `@astronautlabs/amf` | 2 | AMF Decode, AMF Encode |
| `highlight.js` | 2 | Render Markdown, Syntax highlighter |
| `argon2-browser` | 2 | Argon2, Argon2 compare |
| `lz4js` | 2 | LZ4 Compress, LZ4 Decompress |
| `ssdeep.js` | 2 | Compare SSDEEP hashes, SSDEEP |
| `yaml` | 1 | JSON to YAML |
| `handlebars` | 1 | Template |
| `json5` | 1 | JSON Beautify |
| `jsesc` | 1 | Escape string |
| `@alexaltea/capstone-js/dist/capstone.min.js` | 1 | Disassemble ARM |
| `zlibjs/bin/unzip.min.js` | 1 | Unzip |
| `node-md6` | 1 | MD6 |
| `diff` | 1 | Diff |
| `jsonata` | 1 | Jsonata Query |
| `url` | 1 | Parse URI |
| `@noble/hashes/blake3.js` | 1 | BLAKE3 |
| `tesseract.js` | 1 | Optical Character Recognition |
| `avsc` | 1 | Avro to JSON |
| `js-yaml` | 1 | YAML to JSON |
| `ua-parser-js` | 1 | Parse User Agent |
| `lodash/kebabCase.js` | 1 | To Kebab case |
| `flat` | 1 | JSON to CSV |
| `ntlm` | 1 | LM Hash |
| `jq-web` | 1 | Jq |
| `terser` | 1 | JavaScript Minify |
| `crypto-js` | 1 | Derive EVP key |
| `zlibjs/bin/zip.min.js` | 1 | Zip |
| `crypto` | 1 | CipherSaber2 Encrypt |
| `lodash/camelCase.js` | 1 | To Camel case |
| `esprima` | 1 | JavaScript Parser |
| `sql-formatter` | 1 | SQL Beautify |
| `unorm` | 1 | Normalise Unicode |
| `exif-parser` | 1 | Extract EXIF |
| `lodash/snakeCase.js` | 1 | To Snake case |
| `libyara-wasm` | 1 | YARA Rules |
| `jsonpath-plus` | 1 | JPath expression |
| `escodegen` | 1 | JavaScript Beautify |

## Blocked through an internal library

These 21 import nothing outside the reference's own source *directly*, and are
blocked anyway: a library under `src/core/lib` that they reach for pulls a
package of its own, or reaches a third that does.

The taint is transitive and needs a fixpoint, not a single lookup. `JA4` is the
case that proves it: nothing in the operation or in `lib/JA4` names a package,
but `lib/JA4` calls `runHash` from `lib/Hash`, and that is where `crypto-api`
enters. Checking one level below the operation puts JA4 in the reachable
column; checking the closure does not.

| Operation | Internal library | What it pulls |
|---|---|---|
| Convert co-ordinate format | `lib/ConvertCoordinates` | `geodesy` |
| Show on map | `lib/ConvertCoordinates` | `geodesy` |
| HAS-160 | `lib/Hash` | `crypto-api/src/crypto-api.mjs` |
| HASSH Client Fingerprint | `lib/Hash` | `crypto-api/src/crypto-api.mjs` |
| HASSH Server Fingerprint | `lib/Hash` | `crypto-api/src/crypto-api.mjs` |
| JA3 Fingerprint | `lib/Hash` | `crypto-api/src/crypto-api.mjs` |
| JA3S Fingerprint | `lib/Hash` | `crypto-api/src/crypto-api.mjs` |
| Snefru | `lib/Hash` | `crypto-api/src/crypto-api.mjs` |
| JA4 Fingerprint | `lib/JA4` | `crypto-api/src/crypto-api.mjs` |
| JA4Server Fingerprint | `lib/JA4` | `crypto-api/src/crypto-api.mjs` |
| LZString Compress | `lib/LZString` | `lz-string` |
| LZString Decompress | `lib/LZString` | `lz-string` |
| Magic | `lib/Magic` | `chi-squared` |
| Protobuf Decode | `lib/Protobuf` | `protobufjs` |
| Protobuf Encode | `lib/Protobuf` | `protobufjs` |
| Parse TLS record | `lib/Protocol` | `bignumber.js` |
| Parse UDP | `lib/Protocol` | `bignumber.js` |
| Generate QR Code | `lib/QRCode` | `jimp` |
| Parse QR Code | `lib/QRCode` | `jimp` |
| SM2 Decrypt | `lib/SM2` | `crypto-api/src/encoder/hex.mjs` |
| SM2 Encrypt | `lib/SM2` | `crypto-api/src/encoder/hex.mjs` |

## Reachable without any port

These 86 import nothing outside the reference's own source, transitively.
They are limited by effort, not by a dependency, and are where the catalog
grows next.

Three left this list at once, for three reasons, and only one was a port.
`To Base` moved to the arbitrary-precision group: it never imported
`bignumber.js`, it was handed one, so reading the imports had missed it.
`LZNT1 Decompress` and `Parse TLV` had been implemented and never struck off —
which is why the counts above now claim to partition the total exactly, a
claim that fails loudly the next time one of them drifts.

Analyse hash, Ascon MAC, Automated Validation Test Op, Bombe, ChaCha, Change IP format, CipherSaber2 Decrypt, Colossus, Conditional Jump, Convert area, Convert data units, Convert distance, Convert mass, Convert speed, CRC Checksum, CSV to JSON, Detect File Type, Disassemble x86, DNS over HTTPS, ELF Info, Enigma, Extract Audio Metadata, Extract dates, Extract Files, Extract ID3, File Tree, Flask Session Decode, Frequency distribution, Fuzzy Match, Generate all checksums, Generate all hashes, Generate Lorem Ipsum, Generic Code Beautify, Get Time, GOST Hash, Group IP addresses, Haversine distance, HTTP request, IPv6 Transition Addresses, Jump, Label, Lorenz, Multiple Bombe, Numberwang, Parse Ethernet frame, Parse IP range, Parse IPv4 header, Parse SSH Host Key, PHP Deserialize, PHP Serialize, Play Media, P-list Viewer, PRESENT Decrypt, PRESENT Encrypt, Pseudo-Random Prime Generator, Rabbit, RAKE, RC6 Decrypt, RC6 Encrypt, Remove Diacritics, Remove EXIF, Render Image, Render PDF, Return, Salsa20, Scan for Embedded Files, Show Base64 offsets, Shuffle, SIGABA, Sleep, SM4 Decrypt, SM4 Encrypt, Sort, Streebog, Subsection, Tar, TEA Decrypt, TEA Encrypt, Text-Integer Conversion, Twofish Decrypt, Twofish Encrypt, Typex, Untar, XSalsa20, XTEA Decrypt, XTEA Encrypt
