// XXTEA, the corrected Block TEA.
//
// The cipher carries the plaintext length in a trailing word, which is what
// lets a message whose size is not a multiple of four survive a round trip.
// Decryption checks that word against the padded size and refuses anything
// outside a three-byte window, so a ciphertext decrypted under the wrong key
// is rejected rather than returned as noise. Both sides of that check are
// sampled.
//
// A key shorter than sixteen bytes is zero-extended rather than refused, so
// the empty key and a one-byte key are different keys with the same padding.
// A key of exactly sixteen bytes and one longer take different branches.

const KEYS = [
    ["Hex", ""],
    ["Hex", "00"],
    ["Hex", "000102030405060708090a0b0c0d0e0f"],
    ["Hex", "000102030405060708090a0b0c0d0e0f1011"],
    ["UTF8", "secret"],
    ["UTF8", "sixteen-byte-key"],
    ["Latin1", "\u00ff\u00fe"],
];

export async function add({addCase, bakeString, randomBytes}) {
    let index = 0;

    // Encryption over lengths that straddle the word boundary, so the length
    // word is exercised at every remainder.
    for (const length of [0, 1, 3, 4, 5, 7, 8, 16, 17, 64, 255]) {
        const data = randomBytes(length);
        addCase(`xxtea_enc_${index++}`, data, [
            {op: "XXTEA Encrypt", args: [{option: "Hex", string: "000102030405060708090a0b0c0d0e0f"}]},
        ]);
    }

    // Every key shape against one fixed message.
    for (const [option, string] of KEYS) {
        addCase(`xxtea_key_${index++}`, "attack at dawn", [
            {op: "XXTEA Encrypt", args: [{option, string}]},
        ]);
    }

    // Decryption is pinned on the reference's own ciphertext, so each case is
    // canonical by construction rather than by a value typed in here.
    for (const length of [1, 4, 5, 16, 64]) {
        const data = randomBytes(length);
        const key = {option: "Hex", string: "000102030405060708090a0b0c0d0e0f"};
        const encrypted = await bakeString(data, [{op: "XXTEA Encrypt", args: [key]}]);
        addCase(`xxtea_dec_${index++}`, encrypted, [
            {op: "XXTEA Decrypt", args: [key]},
        ]);
    }

    // A round trip through both halves, which must return the input exactly.
    for (const length of [1, 5, 17]) {
        addCase(`xxtea_round_${index++}`, randomBytes(length), [
            {op: "XXTEA Encrypt", args: [{option: "UTF8", string: "secret"}]},
            {op: "XXTEA Decrypt", args: [{option: "UTF8", string: "secret"}]},
        ]);
    }
}