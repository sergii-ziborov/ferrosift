// Automatic differential corpus generator.
//
// Deterministically samples inputs for every CyberChef-aliased FerroSift
// operation, bakes each recipe through the pinned checkout, and records the
// exact output bytes at every recipe prefix. `tests/corpus.rs` replays the
// result and asserts byte-for-byte equality and stopping positions.
//
// Determinism: a seeded xorshift PRNG, no clock, no Math.random. Decoder and
// decompressor inputs derive from the paired reference encoder at generation
// time, so every decode case is canonical by construction.
import {mkdir, writeFile} from "node:fs/promises";
import path from "node:path";
import {deflateRawSync, deflateSync, gzipSync} from "node:zlib";

import {
    COMMIT,
    VERSION,
    bakeBytes as bakeBytesWith,
    bakeHex,
    bakeString as bakeStringWith,
    fixtureDir,
    loadChef,
    makeInput,
} from "./reference.mjs";

const chef = await loadChef();
const output = path.join(fixtureDir, "corpus.json");

const bakeFinalHex = (input, recipe) => bakeHex(chef, input, recipe);
const bakeString = (input, recipe) => bakeStringWith(chef, input, recipe);
const bakeBytes = (input, recipe) => bakeBytesWith(chef, input, recipe);

// Deterministic PRNG (xorshift32) and byte samplers.
// ---------------------------------------------------------------------------
function makeRng(seed) {
    let state = seed >>> 0;
    if (state === 0) state = 0x1a2b3c4d;
    return () => {
        state ^= state << 13;
        state ^= state >>> 17;
        state ^= state << 5;
        state >>>= 0;
        return state / 0x1_0000_0000;
    };
}

const rng = makeRng(0x5f37_1d10);

function randomBytes(length) {
    return Buffer.from(Array.from({length}, () => Math.floor(rng() * 256)));
}

function randomAscii(length) {
    const printable = [];
    for (let i = 0; i < length; i++) {
        printable.push(0x20 + Math.floor(rng() * 0x5f));
    }
    return Buffer.from(printable);
}

// The interesting length classes for block/group codecs: empty, sub-block,
// exact block, block+1, and a couple of larger sizes.
const LENGTHS = [0, 1, 2, 3, 4, 5, 7, 8, 11, 16, 20, 31];


const cases = [];

function addCase(name, input, recipe) {
    cases.push({name, input: encodeInput(input), recipe});
}

function encodeInput(input) {
    return Buffer.isBuffer(input)
        ? {kind: "bytes", hex: input.toString("hex")}
        : {kind: "text", value: input};
}

// An encode/decode pair: sample raw bytes, bake the encoder to a canonical
// string, pin the decoder on that string. Both directions become cases.
async function encodeDecodePair(prefix, encoder, decoder, lengths = LENGTHS) {
    for (const length of lengths) {
        const raw = randomBytes(length);
        addCase(`${prefix}_encode_${length}`, raw, [encoder]);
        const encoded = await bakeString(raw, [encoder]);
        addCase(`${prefix}_decode_${length}`, encoded, [decoder]);
    }
}

// A byte-to-text transform sampled over raw bytes (encode direction only).
function textForm(prefix, recipe, lengths = LENGTHS) {
    for (const length of lengths) {
        addCase(`${prefix}_${length}`, randomBytes(length), recipe);
    }
}

// ---------------------------------------------------------------------------
// Encoding families (round trips, canonical decode).
// ---------------------------------------------------------------------------
const B32 = "A-Z2-7=";
const B32HEX = "0-9A-V=";
const B45 = "0-9A-Z $%*+\\-./:";
const B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const B58R = "rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";
const B64 = "A-Za-z0-9+/=";
const B64URL = "A-Za-z0-9-_";
const B85 = "!-u";
const B85Z = "0-9a-zA-Z.\\-:+=^!/*?&<>()[]{}@%$#";

await encodeDecodePair(
    "base32",
    {op: "To Base32", args: [B32]},
    {op: "From Base32", args: [B32, true]},
);
await encodeDecodePair(
    "base32hex",
    {op: "To Base32", args: [B32HEX]},
    {op: "From Base32", args: [B32HEX, true]},
);
await encodeDecodePair(
    "base45",
    {op: "To Base45", args: [B45]},
    {op: "From Base45", args: [B45, true]},
);
await encodeDecodePair(
    "base58",
    {op: "To Base58", args: [B58]},
    {op: "From Base58", args: [B58, true]},
);
await encodeDecodePair(
    "base58ripple",
    {op: "To Base58", args: [B58R]},
    {op: "From Base58", args: [B58R, true]},
);
await encodeDecodePair(
    "base64",
    {op: "To Base64", args: [B64]},
    {op: "From Base64", args: [B64, true, false]},
);
await encodeDecodePair(
    "base64url",
    {op: "To Base64", args: [B64URL]},
    {op: "From Base64", args: [B64URL, true, false]},
);
await encodeDecodePair(
    "base85",
    {op: "To Base85", args: [B85, false]},
    {op: "From Base85", args: [B85, true, "z"]},
);
await encodeDecodePair(
    "base85z85",
    {op: "To Base85", args: [B85Z, false]},
    // Z85 contains "z", so the zero-group character must be empty to avoid the
    // alphabet-conflict rejection (which FerroSift reproduces identically).
    {op: "From Base85", args: [B85Z, true, ""]},
);
await encodeDecodePair(
    "hex",
    {op: "To Hex", args: ["Space", 0]},
    {op: "From Hex", args: ["Auto"]},
);
await encodeDecodePair(
    "hex0x",
    {op: "To Hex", args: ["0x with comma", 0]},
    {op: "From Hex", args: ["0x with comma"]},
);
await encodeDecodePair(
    "binary",
    {op: "To Binary", args: ["Space", 8]},
    {op: "From Binary", args: ["Space", 8]},
);
await encodeDecodePair(
    "decimal",
    {op: "To Decimal", args: ["Space", false]},
    {op: "From Decimal", args: ["Space", false]},
);
await encodeDecodePair(
    "octal",
    {op: "To Octal", args: ["Space"]},
    {op: "From Octal", args: ["Space"]},
);
// To Charcode is text-to-text (not byte input), so sample printable text.
for (const length of LENGTHS) {
    const raw = randomAscii(length).toString("latin1");
    const encoder = {op: "To Charcode", args: ["Space", 16]};
    addCase(`charcode_encode_${length}`, raw, [encoder]);
    const encoded = await bakeString(raw, [encoder]);
    addCase(`charcode_decode_${length}`, encoded, [
        {op: "From Charcode", args: ["Space", 16]},
    ]);
}

// ---------------------------------------------------------------------------
// URL, HTML, ROT13 (text transforms over printable ASCII).
// ---------------------------------------------------------------------------
for (const length of [0, 1, 4, 8, 16, 24]) {
    const raw = randomBytes(length);
    addCase(`url_encode_${length}`, raw, [{op: "URL Encode", args: [false]}]);
    addCase(`url_encode_all_${length}`, raw, [{op: "URL Encode", args: [true]}]);
    const encoded = await bakeString(raw, [{op: "URL Encode", args: [false]}]);
    addCase(`url_decode_${length}`, encoded, [{op: "URL Decode", args: [true]}]);
}
// URL decode legacy-fallback and unicode-escape paths.
for (const value of ["%E0%A4%A", "%FF%FE", "%u0413%u0414", "100%", "a+b%2Bc", "%C3%28"]) {
    addCase(`url_decode_edge_${cases.length}`, value, [{op: "URL Decode", args: [true]}]);
}

for (const length of [0, 4, 12, 24]) {
    // To/From HTML Entity are text-to-text; feed printable text, not bytes.
    // Named-entity encoding is a documented subset divergence (FerroSift emits
    // the classic entity set, CyberChef the full HTML5 table), so the corpus
    // pins the numeric-entity encode path and decoding of numeric entities.
    const raw = randomAscii(length).toString("latin1");
    const numeric = {op: "To HTML Entity", args: [true, "Numeric entities"]};
    addCase(`html_numeric_${length}`, raw, [numeric]);
    const encoded = await bakeString(raw, [numeric]);
    addCase(`html_from_numeric_${length}`, encoded, [{op: "From HTML Entity", args: []}]);
}

for (const length of [0, 5, 13, 26]) {
    addCase(`rot13_${length}`, randomAscii(length), [
        {op: "ROT13", args: [true, true, false, 13]},
    ]);
}

// ---------------------------------------------------------------------------
// Hexdump (round trip and width/flag variants).
// ---------------------------------------------------------------------------
for (const length of [0, 1, 8, 16, 31]) {
    const raw = randomBytes(length);
    addCase(`hexdump_${length}`, raw, [
        {op: "To Hexdump", args: [16, false, false, false]},
        {op: "From Hexdump", args: []},
    ]);
    addCase(`hexdump_upper_${length}`, raw, [
        {op: "To Hexdump", args: [8, true, true, false]},
    ]);
}

// ---------------------------------------------------------------------------
// XOR (all schemes, hex/utf8 keys, null preservation).
// ---------------------------------------------------------------------------
for (const scheme of ["Standard", "Input differential", "Output differential", "Cascade"]) {
    for (const length of [0, 1, 5, 16]) {
        addCase(`xor_${scheme.split(" ")[0].toLowerCase()}_${length}`, randomBytes(length), [
            {op: "XOR", args: [{option: "Hex", string: "3f"}, scheme, false]},
        ]);
    }
}
for (const length of [4, 9]) {
    addCase(`xor_null_${length}`, randomBytes(length), [
        {op: "XOR", args: [{option: "Hex", string: "01"}, "Standard", true]},
    ]);
    addCase(`xor_utf8key_${length}`, randomBytes(length), [
        {op: "XOR", args: [{option: "UTF8", string: "key"}, "Standard", false]},
    ]);
}
// XOR Brute Force emits a deterministic multi-line report; pin it directly.
for (const [name, hex] of [["a", "1f001a1b00"], ["b", "48656c6c6f"]]) {
    addCase(`xor_brute_${name}`, Buffer.from(hex, "hex"), [
        {op: "XOR Brute Force", args: [1, 100, 0, "Standard", false, true, false, ""]},
    ]);
}

// ---------------------------------------------------------------------------
// Hashes and HMAC.
// ---------------------------------------------------------------------------
for (const length of [0, 1, 16, 55, 64, 120]) {
    const raw = randomBytes(length);
    addCase(`md5_${length}`, raw, [{op: "MD5", args: []}]);
    addCase(`sha1_${length}`, raw, [{op: "SHA1", args: [80]}]);
    addCase(`sha2_256_${length}`, raw, [{op: "SHA2", args: ["256", 64, 160]}]);
    addCase(`sha2_512_${length}`, raw, [{op: "SHA2", args: ["512", 64, 160]}]);
    addCase(`sha3_256_${length}`, raw, [{op: "SHA3", args: ["256"]}]);
    addCase(`sha3_512_${length}`, raw, [{op: "SHA3", args: ["512"]}]);
}
for (const hasher of ["MD5", "SHA1", "SHA256", "SHA512"]) {
    addCase(`hmac_${hasher.toLowerCase()}`, randomBytes(20), [
        {op: "HMAC", args: [{option: "UTF8", string: "ferro-key"}, hasher]},
    ]);
}

// ---------------------------------------------------------------------------
// AES / RC4 (encrypt sampled; decrypt derived from ciphertext).
// ---------------------------------------------------------------------------
const AES_KEY = "00112233445566778899aabbccddeeff";
const AES_IV = "0102030405060708090a0b0c0d0e0f10";
for (const mode of ["CBC", "CFB", "OFB", "CTR"]) {
    const plain = randomBytes(32);
    const enc = {
        op: "AES Encrypt",
        args: [
            {option: "Hex", string: AES_KEY},
            {option: "Hex", string: AES_IV},
            mode,
            "Raw",
            "Hex",
            {option: "Hex", string: ""},
            "Off",
        ],
    };
    addCase(`aes_${mode.toLowerCase()}_encrypt`, plain, [enc]);
    const cipher = await bakeString(plain, [enc]);
    addCase(`aes_${mode.toLowerCase()}_decrypt`, cipher, [
        {
            op: "AES Decrypt",
            args: [
                {option: "Hex", string: AES_KEY},
                {option: "Hex", string: AES_IV},
                16,
                mode,
                "Hex",
                "Hex",
                {option: "Hex", string: ""},
                {option: "Hex", string: ""},
                "Off",
            ],
        },
    ]);
}
// AES Key Wrap / Unwrap (RFC 3394).
{
    const kek = "000102030405060708090a0b0c0d0e0f";
    const wrapped = await bakeString("00112233445566778899aabbccddeeff", [
        {
            op: "AES Key Wrap",
            args: [
                {option: "Hex", string: kek},
                {option: "Hex", string: "a6a6a6a6a6a6a6a6"},
                "Hex",
                "Hex",
            ],
        },
    ]);
    addCase("aes_key_wrap", "00112233445566778899aabbccddeeff", [
        {
            op: "AES Key Wrap",
            args: [
                {option: "Hex", string: kek},
                {option: "Hex", string: "a6a6a6a6a6a6a6a6"},
                "Hex",
                "Hex",
            ],
        },
    ]);
    addCase("aes_key_unwrap", wrapped, [
        {
            op: "AES Key Unwrap",
            args: [
                {option: "Hex", string: kek},
                {option: "Hex", string: "a6a6a6a6a6a6a6a6"},
                "Hex",
                "Hex",
            ],
        },
    ]);
}
for (const length of [1, 5, 16]) {
    addCase(`rc4_${length}`, randomBytes(length), [
        {op: "RC4", args: [{option: "UTF8", string: "secret"}, "Latin1", "Hex"]},
    ]);
}

// ---------------------------------------------------------------------------
// KDF (kept small â€” deliberately low cost parameters).
// ---------------------------------------------------------------------------
addCase("pbkdf2_sha256", "", [
    {
        op: "Derive PBKDF2 key",
        args: [
            {option: "UTF8", string: "password"},
            128,
            1,
            "SHA256",
            {option: "UTF8", string: "saltsalt"},
        ],
    },
]);
addCase("scrypt_low_cost", "password", [
    {op: "Scrypt", args: [{option: "UTF8", string: "saltsalt"}, 16, 1, 1, 32]},
]);

// ---------------------------------------------------------------------------
// Compression: the inflate direction is byte-pinned. Inputs are standard
// DEFLATE streams produced by Node's own zlib (the CyberChef node build's
// deflate operations abort under Node 24), so CyberChef's inflate output is
// pinned against a producer-independent stream. Compression-direction parity
// is interoperable, not bit-identical, and is exercised by the wave
// round-trip tests plus the pinned Bzip2 payload in differential.json.
for (const length of [0, 1, 32, 200]) {
    const raw = randomBytes(length);
    addCase(`gunzip_${length}`, gzipSync(raw), [{op: "Gunzip", args: []}]);
    addCase(`zlib_inflate_${length}`, deflateSync(raw), [
        {op: "Zlib Inflate", args: [0, 0, "Adaptive", false, false]},
    ]);
    addCase(`raw_inflate_${length}`, deflateRawSync(raw), [
        {op: "Raw Inflate", args: [0, 0, "Adaptive", false, false]},
    ]);
}

// ---------------------------------------------------------------------------
// Data slicing and text head.
// ---------------------------------------------------------------------------
for (const [start, length, name] of [[0, 4, "a"], [2, 3, "b"], [3, 0, "c"]]) {
    addCase(`take_bytes_${name}`, randomBytes(12), [
        {op: "Take bytes", args: [start, length, false]},
    ]);
    addCase(`drop_bytes_${name}`, randomBytes(12), [
        {op: "Drop bytes", args: [start, length, false]},
    ]);
}
for (const n of [1, 2, 3, -1]) {
    addCase(`head_${n}`, "l1\nl2\nl3\nl4\nl5", [{op: "Head", args: ["Line feed", n]}]);
}
addCase("find_replace_simple", "foo\tbar foo baz", [
    {
        op: "Find / Replace",
        args: [{option: "Simple string", string: "foo"}, "X", true, false, true, false],
    },
]);
addCase("find_replace_regex", "a1b2c3d4", [
    {
        op: "Find / Replace",
        args: [{option: "Regex", string: "[0-9]"}, "#", true, false, true, false],
    },
]);
addCase("strings_ascii", "\u0000\u0000Hello World\u0000AB\u0000secret_value", [
    {op: "Strings", args: ["Single byte", 4, "All printable chars (A)", false, false, false]},
]);

// ---------------------------------------------------------------------------
// Extractors and defang (deterministic corpora built from fixed fixtures).
// ---------------------------------------------------------------------------
const IOC_TEXT =
    "Contact admin@example.com or ops@corp.example.org, visit " +
    "https://evil.example/path?x=1 and http://a.b.example see 8.8.8.8, " +
    "192.168.1.1 and 2001:db8::1 with domain.example.org and " +
    "aa:bb:cc:dd:ee:ff plus AA-BB-CC-DD-EE-FF and path C:\\Windows\\cmd.exe " +
    "and /usr/bin/python3 and hash 0123456789abcdef0123456789abcdef01234567";
addCase("extract_ip", IOC_TEXT, [
    {op: "Extract IP addresses", args: [true, true, false, false, false, false]},
]);
addCase("extract_urls", IOC_TEXT, [{op: "Extract URLs", args: [false, false, false]}]);
addCase("extract_emails", IOC_TEXT, [
    {op: "Extract email addresses", args: [false, false, false]},
]);
addCase("extract_domains", IOC_TEXT, [
    {op: "Extract domains", args: [false, false, false, false]},
]);
addCase("extract_mac", IOC_TEXT, [{op: "Extract MAC addresses", args: [true, true, true]}]);
addCase("extract_hashes", IOC_TEXT, [{op: "Extract hashes", args: [40, false, false]}]);
addCase("extract_file_paths", IOC_TEXT, [
    {op: "Extract file paths", args: [true, true, false, false, false]},
]);
addCase("defang_url", "https://evil.example/path?x=1", [
    {op: "Defang URL", args: [true, true, true, "Only full URLs"]},
]);
addCase("defang_and_fang_url", "https://evil.example/a", [
    {op: "Defang URL", args: [true, true, true, "Only full URLs"]},
    {op: "Fang URL", args: [true, true, true]},
]);
addCase("defang_ip", "8.8.8.8 and 1.2.3.4 and 2001:db8::1", [
    {op: "Defang IP Addresses", args: []},
]);

// ---------------------------------------------------------------------------
// Bake every case at every prefix.
// ---------------------------------------------------------------------------
let failures = 0;
for (const testCase of cases) {
    testCase.outputs_hex = [];
    for (let length = 1; length <= testCase.recipe.length; length += 1) {
        try {
            testCase.outputs_hex.push(
                await bakeFinalHex(makeInput(testCase.input), testCase.recipe.slice(0, length)),
            );
        } catch (error) {
            failures += 1;
            process.stderr.write(
                `bake failed: ${testCase.name} prefix ${length}: ${error?.message ?? error}\n`,
            );
            break;
        }
    }
    testCase.stopped_after = testCase.outputs_hex.length;
}

const complete = cases.filter(
    testCase => testCase.stopped_after === testCase.recipe.length,
);

const suite = {
    reference: {name: "CyberChef", version: VERSION, commit: COMMIT},
    cases: complete,
};

await mkdir(path.dirname(output), {recursive: true});
await writeFile(output, `${JSON.stringify(suite, null, 1)}\n`, "utf8");
process.stdout.write(
    `wrote ${complete.length} corpus cases (${failures} bake failures dropped) to ${output}\n`,
);

