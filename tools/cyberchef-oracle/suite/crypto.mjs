// Ciphers, key derivation, and key wrapping.

export const cryptoCases = [
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
];
