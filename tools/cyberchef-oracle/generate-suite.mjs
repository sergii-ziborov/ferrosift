// Curated CyberChef differential suite.
//
// Hand-picked recipes that exercise representative and quirk-prone paths.
// Each case records the reference output at every recipe prefix, so a
// divergence is reported at the step that caused it.
import {mkdir, writeFile} from "node:fs/promises";
import path from "node:path";

import {
    COMMIT,
    VERSION,
    bakeEveryPrefix,
    fixtureDir,
    loadChef,
} from "./reference.mjs";

const chef = await loadChef();
const output = path.join(fixtureDir, "differential.json");

const standard = "A-Za-z0-9+/=";
const crypt = "/128GhIoPQROSTeUbADfgHijKLM+n0pFWXY456xyzB7=39VaqrstJklmNuZvwcdEC";
const allBytes = Array.from({length: 256}, (_, value) => value)
    .map(value => value.toString(16).padStart(2, "0"))
    .join("");
const cases = [
    {
        name: "to_hex_ascii_space",
        input: {kind: "bytes", hex: "466572726f53696674"},
        recipe: [{op: "To Hex", args: ["Space", 0]}],
    },
    {
        name: "to_hex_binary_colon_lines",
        input: {kind: "bytes", hex: "000f10ffaac3"},
        recipe: [{op: "To Hex", args: ["Colon", 2]}],
    },
    {
        name: "from_hex_auto_mixed",
        input: {kind: "text", value: "0x00, 0f:10\nff"},
        recipe: [{op: "From Hex", args: ["Auto"]}],
    },
    {
        name: "hex_percent_round_trip",
        input: {kind: "bytes", hex: "00ff107f"},
        recipe: [
            {op: "To Hex", args: ["Percent", 0]},
            {op: "From Hex", args: ["Percent"]},
        ],
    },
    {
        name: "to_base64_utf8",
        input: {kind: "bytes", hex: "ce93ceb5ceb9ceac20cf83cebfcf85"},
        recipe: [{op: "To Base64", args: [standard]}],
    },
    {
        name: "to_base64_url_safe_unpadded",
        input: {kind: "bytes", hex: "fbff0001"},
        recipe: [{op: "To Base64", args: ["A-Za-z0-9-_"]}],
    },
    {
        name: "from_base64_noise",
        input: {kind: "text", value: " Zm9v\n"},
        recipe: [{op: "From Base64", args: [standard, true, false]}],
    },
    {
        name: "base64_standard_round_trip_all_bytes",
        input: {kind: "bytes", hex: allBytes},
        recipe: [
            {op: "To Base64", args: [standard]},
            {op: "From Base64", args: [standard, true, false]},
        ],
    },
    {
        name: "base64_crypt_round_trip",
        input: {
            kind: "bytes",
            hex: "466572726f5369667420637573746f6d20616c706861626574",
        },
        recipe: [
            {op: "To Base64", args: [crypt]},
            {op: "From Base64", args: [crypt, true, false]},
        ],
    },
    {
        name: "base64_and_hex_composition",
        input: {kind: "bytes", hex: "00466572726f53696674ff"},
        recipe: [
            {op: "To Base64", args: [standard]},
            {op: "From Base64", args: [standard, true, false]},
            {op: "To Hex", args: ["0x with comma", 0]},
            {op: "From Hex", args: ["0x with comma"]},
        ],
    },
    {
        name: "base32_standard_round_trip_all_bytes",
        input: {kind: "bytes", hex: allBytes},
        recipe: [
            {op: "To Base32", args: ["A-Z2-7="]},
            {op: "From Base32", args: ["A-Z2-7=", true]},
        ],
    },
    {
        name: "base32_unpadded_alphabet_round_trip",
        input: {kind: "bytes", hex: "466572726f5369667421"},
        recipe: [
            {op: "To Base32", args: ["A-Z2-7"]},
            {op: "From Base32", args: ["A-Z2-7", true]},
        ],
    },
    {
        name: "from_base32_hex_extended_noise",
        input: {kind: "text", value: "CPNMU===\n"},
        recipe: [{op: "From Base32", args: ["0-9A-V=", true]}],
    },
    {
        name: "base45_round_trip",
        input: {kind: "bytes", hex: "466572726f536966742100"},
        recipe: [
            {op: "To Base45", args: ["0-9A-Z $%*+\\-./:"]},
            {op: "From Base45", args: ["0-9A-Z $%*+\\-./:", true]},
        ],
    },
    {
        name: "base58_round_trip_leading_zeros",
        input: {kind: "bytes", hex: "000001ff"},
        recipe: [
            {
                op: "To Base58",
                args: ["123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"],
            },
            {
                op: "From Base58",
                args: [
                    "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz",
                    true,
                ],
            },
        ],
    },
    {
        name: "base85_delimited_zero_groups_round_trip",
        input: {kind: "bytes", hex: "0000000048656c6c6f00"},
        recipe: [
            {op: "To Base85", args: ["!-u", true]},
            {op: "From Base85", args: ["!-u", true, "z"]},
        ],
    },
    {
        name: "to_base85_z85_alphabet",
        input: {kind: "bytes", hex: "68656c6c6f20776f726c64"},
        recipe: [
            {op: "To Base85", args: ["0-9a-zA-Z.\\-:+=^!/*?&<>()[]{}@%$#", false]},
        ],
    },
    {
        name: "url_percent_round_trip",
        input: {kind: "bytes", hex: "68656c6c6f20776f726c642f3f235b5dff00"},
        recipe: [
            {op: "URL Encode", args: [false]},
            {op: "URL Decode", args: [true]},
        ],
    },
    {
        name: "url_decode_fallback_path",
        input: {kind: "text", value: "%E0%A4%A"},
        recipe: [{op: "URL Decode", args: [true]}],
    },
    {
        name: "url_decode_multibyte_utf8",
        input: {kind: "text", value: "%CE%93%CE%B5%CE%B9%CE%AC+%u0414"},
        recipe: [{op: "URL Decode", args: [true]}],
    },
    {
        name: "binary_colon_round_trip",
        input: {kind: "bytes", hex: "00ff0a80"},
        recipe: [
            {op: "To Binary", args: ["Colon", 8]},
            {op: "From Binary", args: ["Colon", 8]},
        ],
    },
    {
        name: "from_binary_mixed_whitespace",
        input: {kind: "text", value: "00001010\t00010100\n00011110"},
        recipe: [{op: "From Binary", args: ["Space", 8]}],
    },
    {
        name: "decimal_signed_round_trip",
        input: {kind: "bytes", hex: "80ff007f"},
        recipe: [
            {op: "To Decimal", args: ["Space", true]},
            {op: "From Decimal", args: ["Space", true]},
        ],
    },
    {
        name: "from_decimal_auto_delimiters",
        input: {kind: "text", value: "72, 101; 108\n111"},
        recipe: [{op: "From Decimal", args: ["Auto", false]}],
    },
    {
        name: "octal_comma_round_trip",
        input: {kind: "bytes", hex: "00ff07c3"},
        recipe: [
            {op: "To Octal", args: ["Comma"]},
            {op: "From Octal", args: ["Comma"]},
        ],
    },
    {
        name: "encoding_families_compose",
        input: {kind: "bytes", hex: "00466572726f53696674ff"},
        recipe: [
            {op: "To Base32", args: ["A-Z2-7="]},
            {op: "From Base32", args: ["A-Z2-7=", true]},
            {op: "To Base85", args: ["!-u", false]},
            {op: "From Base85", args: ["!-u", true, "z"]},
            {op: "To Octal", args: ["Space"]},
            {op: "From Octal", args: ["Space"]},
        ],
    },
    {
        name: "xor_hex_standard",
        input: {kind: "bytes", hex: "48656c6c6f"},
        recipe: [{op: "XOR", args: [{option: "Hex", string: "0f"}, "Standard", false]}],
    },
    {
        name: "xor_null_preserving",
        input: {kind: "bytes", hex: "00010002"},
        recipe: [{op: "XOR", args: [{option: "Hex", string: "01"}, "Standard", true]}],
    },
    {
        name: "xor_cascade",
        input: {kind: "bytes", hex: "01020304"},
        recipe: [{op: "XOR", args: [{option: "Hex", string: "ff"}, "Cascade", false]}],
    },
    {
        name: "hexdump_round_trip",
        input: {kind: "bytes", hex: "466572726f53696674"},
        recipe: [
            {op: "To Hexdump", args: [16, false, false, false]},
            {op: "From Hexdump", args: []},
        ],
    },
    {
        name: "to_hexdump_upper_final",
        input: {kind: "bytes", hex: "4142"},
        recipe: [{op: "To Hexdump", args: [8, true, true, false]}],
    },
    {
        name: "take_and_drop_bytes",
        input: {kind: "bytes", hex: "6162636465666768"},
        recipe: [
            {op: "Take bytes", args: [2, 3, false]},
            {op: "Drop bytes", args: [1, 1, false]},
        ],
    },
    {
        name: "head_line_feed",
        input: {kind: "text", value: "a\nb\nc\nd"},
        recipe: [{op: "Head", args: ["Line feed", 2]}],
    },
    {
        name: "find_replace_simple_and_extended",
        input: {kind: "text", value: "foo\tbar foo"},
        recipe: [
            {
                op: "Find / Replace",
                args: [
                    {option: "Simple string", string: "foo"},
                    "x",
                    true,
                    false,
                    true,
                    false,
                ],
            },
            {
                op: "Find / Replace",
                args: [
                    {option: "Extended (\\n, \\t, \\x...)", string: "\\t"},
                    "-",
                    true,
                    false,
                    true,
                    false,
                ],
            },
        ],
    },
    {
        name: "gunzip_hello",
        input: {
            kind: "bytes",
            hex: "1f8b080000000000000acb48cdc9c957484b2d2aca070084a9e47c0b000000",
        },
        recipe: [{op: "Gunzip", args: []}],
    },
    {
        name: "md5_hello",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [{op: "MD5", args: []}],
    },
    {
        name: "sha1_hello",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [{op: "SHA1", args: [80]}],
    },
    {
        name: "sha2_256_hello",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [{op: "SHA2", args: ["256", 64, 160]}],
    },
    {
        name: "hmac_sha256_utf8_key",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [
            {
                op: "HMAC",
                args: [{option: "UTF8", string: "key"}, "SHA256"],
            },
        ],
    },
    {
        name: "zlib_inflate_hello",
        // Pinned CyberChef Zlib Deflate of "hello" (Dynamic Huffman Coding).
        input: {kind: "bytes", hex: "789c0580b105000000c3ae2d1d42fedfe20303062c0215"},
        recipe: [{op: "Zlib Inflate", args: [0, 0, "Adaptive", false, false]}],
    },
    {
        name: "html_entity_round_trip",
        input: {kind: "text", value: "a & b <c>"},
        recipe: [
            {op: "To HTML Entity", args: [false, "Named entities"]},
            {op: "From HTML Entity", args: []},
        ],
    },
    {
        name: "rot13_hello_world",
        input: {kind: "bytes", hex: "48656c6c6f2c20576f726c6421"},
        recipe: [{op: "ROT13", args: [true, true, false, 13]}],
    },
    {
        name: "charcode_round_trip",
        input: {kind: "text", value: "Hi"},
        recipe: [
            {op: "To Charcode", args: ["Space", 16]},
            {op: "From Charcode", args: ["Space", 16]},
        ],
    },
    {
        name: "extract_ip_url_email",
        input: {
            kind: "text",
            value:
                "Contact admin@example.com or visit https://evil.example/path?x=1 see 8.8.8.8 and 192.168.1.1 also domain.example.org",
        },
        recipe: [
            {op: "Extract IP addresses", args: [true, false, false, false, false, false]},
        ],
    },
    {
        name: "extract_urls",
        input: {
            kind: "text",
            value:
                "Contact admin@example.com or visit https://evil.example/path?x=1 see 8.8.8.8",
        },
        recipe: [{op: "Extract URLs", args: [false, false, false]}],
    },
    {
        name: "extract_emails",
        input: {
            kind: "text",
            value: "Contact admin@example.com or visit https://evil.example/path",
        },
        recipe: [{op: "Extract email addresses", args: [false, false, false]}],
    },
    {
        name: "defang_and_fang_url",
        input: {kind: "text", value: "https://evil.example/path"},
        recipe: [
            {
                op: "Defang URL",
                args: [true, true, true, "Only full URLs"],
            },
            {op: "Fang URL", args: [true, true, true]},
        ],
    },
    {
        name: "defang_ip_addresses",
        input: {kind: "text", value: "8.8.8.8 and 1.2.3.4"},
        recipe: [{op: "Defang IP Addresses", args: []}],
    },
    {
        name: "strings_ascii_printable",
        input: {kind: "text", value: "\u0000\u0000Hello World\u0000\u0000test\u0000AB"},
        recipe: [
            {
                op: "Strings",
                args: ["Single byte", 4, "All printable chars (A)", false, false, false],
            },
        ],
    },
    {
        name: "aes_cbc_encrypt_hex",
        input: {kind: "text", value: "Attack at dawn!!"},
        recipe: [
            {
                op: "AES Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "00000000000000000000000000000000"},
                    "CBC",
                    "Raw",
                    "Hex",
                    {option: "Hex", string: ""},
                    "Off",
                ],
            },
        ],
    },
    {
        name: "aes_cbc_round_trip",
        input: {kind: "text", value: "Attack at dawn!!"},
        recipe: [
            {
                op: "AES Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "00000000000000000000000000000000"},
                    "CBC",
                    "Raw",
                    "Hex",
                    {option: "Hex", string: ""},
                    "Off",
                ],
            },
            {
                op: "AES Decrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "00000000000000000000000000000000"},
                    16,
                    "CBC",
                    "Hex",
                    "Raw",
                    {option: "Hex", string: ""},
                    {option: "Hex", string: ""},
                    "Off",
                ],
            },
        ],
    },
    {
        name: "rc4_utf8_to_hex",
        input: {kind: "text", value: "Hello"},
        recipe: [
            {
                op: "RC4",
                args: [{option: "UTF8", string: "secret"}, "UTF8", "Hex"],
            },
        ],
    },
    {
        name: "xor_brute_force_sample",
        input: {kind: "bytes", hex: "1f001a1b00"},
        recipe: [
            {
                op: "XOR Brute Force",
                args: [1, 5, 0, "Standard", false, true, true, ""],
            },
        ],
    },
    {
        name: "aes_cfb_encrypt_hex",
        input: {kind: "text", value: "Attack at dawn!! more bytes here!!!"},
        recipe: [
            {
                op: "AES Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "0102030405060708090a0b0c0d0e0f10"},
                    "CFB",
                    "Raw",
                    "Hex",
                    {option: "Hex", string: ""},
                    "Off",
                ],
            },
        ],
    },
    {
        name: "aes_ofb_encrypt_hex",
        input: {kind: "text", value: "Attack at dawn!! more bytes here!!!"},
        recipe: [
            {
                op: "AES Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "0102030405060708090a0b0c0d0e0f10"},
                    "OFB",
                    "Raw",
                    "Hex",
                    {option: "Hex", string: ""},
                    "Off",
                ],
            },
        ],
    },
    {
        name: "aes_ctr_encrypt_hex",
        input: {kind: "text", value: "Attack at dawn!! more bytes here!!!"},
        recipe: [
            {
                op: "AES Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "0102030405060708090a0b0c0d0e0f10"},
                    "CTR",
                    "Raw",
                    "Hex",
                    {option: "Hex", string: ""},
                    "Off",
                ],
            },
        ],
    },
    {
        name: "aes_key_wrap_rfc3394",
        input: {kind: "text", value: "00112233445566778899aabbccddeeff"},
        recipe: [
            {
                op: "AES Key Wrap",
                args: [
                    {option: "Hex", string: "000102030405060708090a0b0c0d0e0f"},
                    {option: "Hex", string: "a6a6a6a6a6a6a6a6"},
                    "Hex",
                    "Hex",
                ],
            },
        ],
    },
    {
        name: "sha3_256_ferrosift",
        input: {kind: "bytes", hex: "466572726f53696674"},
        recipe: [{op: "SHA3", args: ["256"]}],
    },
    {
        name: "pbkdf2_sha256_fixed_salt",
        input: {kind: "text", value: ""},
        recipe: [
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
        ],
    },
    {
        name: "scrypt_low_cost",
        input: {kind: "text", value: "password"},
        recipe: [
            {
                op: "Scrypt",
                args: [
                    {option: "UTF8", string: "saltsalt"},
                    16,
                    1,
                    1,
                    32,
                ],
            },
        ],
    },
    {
        // Compress first so the fixture pins CyberChef ciphertext; FerroSift
        // only needs to match the final inflate (compress is interoperable,
        // not bit-identical). Differential runner still checks every prefix,
        // so this case is import+final only via a decompress-only recipe.
        name: "raw_inflate_cyberchef_payload",
        input: {
            kind: "bytes",
            hex: "0540c1090020105aa5696e814608031f6568eb3f8e82adc9fdc7d2794642dd06",
        },
        recipe: [{op: "Raw Inflate", args: [0, 0, "Adaptive", false, false]}],
    },
    {
        name: "bzip2_decompress_cyberchef_payload",
        input: {
            kind: "bytes",
            hex: "425a683931415926535911be4bc300000097804000010008000b23dc0020003100000843d469e93ca337a0ae9f201da61561fc5dc914e1424046f92f0c",
        },
        recipe: [{op: "Bzip2 Decompress", args: [false]}],
    },
    {
        name: "extract_mac_addresses_sorted_unique",
        input: {
            kind: "text",
            value: "Hosts aa:bb:cc:dd:ee:ff and AA-BB-CC-DD-EE-FF and aa:bb:cc:dd:ee:ff and 11:22:33:44:55:66",
        },
        recipe: [{op: "Extract MAC addresses", args: [true, true, true]}],
    },
    {
        name: "extract_hashes_sha1_length",
        input: {
            kind: "text",
            value: "md5 deadbeefcafebabe0123456789abcdef and sha1 0123456789abcdef0123456789abcdef01234567 and again 0123456789abcdef0123456789abcdef01234567",
        },
        recipe: [{op: "Extract hashes", args: [40, false, false]}],
    },
    {
        name: "extract_file_paths_win_unix",
        input: {
            kind: "text",
            value: "See C:\\Windows\\System32\\cmd.exe and /usr/bin/python3.11 and C:\\Temp\\file.txt",
        },
        recipe: [{op: "Extract file paths", args: [true, true, false, false, false]}],
    },
];

for (const testCase of cases) {
    try {
        testCase.outputs_hex = await bakeEveryPrefix(chef, testCase);
    } catch (error) {
        throw new Error(`${testCase.name} failed to bake`, {cause: error});
    }
    testCase.stopped_after = testCase.outputs_hex.length;
}

const suite = {
    reference: {name: "CyberChef", version: VERSION, commit: COMMIT},
    cases,
    unsupported: {
        name: "magic_is_explicitly_unsupported",
        recipe: [{op: "Magic", args: []}],
        finding: {
            code: "compat.cyberchef.unknown_operation",
            source_step: 0,
            original_operation: "Magic",
        },
    },
};

await mkdir(path.dirname(output), {recursive: true});
await writeFile(output, `${JSON.stringify(suite, null, 2)}\n`, "utf8");
process.stdout.write(`wrote ${cases.length} cases to ${output}\n`);
