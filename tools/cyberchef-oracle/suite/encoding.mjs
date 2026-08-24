// Encoding: radix, base-N, URL, and their compositions.

import {allBytes, crypt, standard} from "./alphabets.mjs";

export const encodingCases = [
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
];
