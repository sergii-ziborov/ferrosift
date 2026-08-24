// AES, RC4, key wrapping, and key derivation.
//
// Encryption is sampled; decryption is pinned on the reference's own
// ciphertext, so a decrypt case can never drift from its encrypt case.

const AES_KEY = "00112233445566778899aabbccddeeff";
const AES_IV = "0102030405060708090a0b0c0d0e0f10";
const KEK = "000102030405060708090a0b0c0d0e0f";
const KEY_WRAP_IV = "a6a6a6a6a6a6a6a6";

const keyWrapArgs = operation => ({
    op: operation,
    args: [{option: "Hex", string: KEK}, {option: "Hex", string: KEY_WRAP_IV}, "Hex", "Hex"],
});

export async function add({addCase, bakeString, randomBytes}) {
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
    const wrapped = await bakeString(AES_KEY, [keyWrapArgs("AES Key Wrap")]);
    addCase("aes_key_wrap", AES_KEY, [keyWrapArgs("AES Key Wrap")]);
    addCase("aes_key_unwrap", wrapped, [keyWrapArgs("AES Key Unwrap")]);

    for (const length of [1, 5, 16]) {
        addCase(`rc4_${length}`, randomBytes(length), [
            {op: "RC4", args: [{option: "UTF8", string: "secret"}, "Latin1", "Hex"]},
        ]);
    }

    // Key derivation, deliberately at low cost parameters.
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
}
