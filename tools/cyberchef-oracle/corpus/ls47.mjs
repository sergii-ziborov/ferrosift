// LS47, a 7x7 widening of the ElsieFour hand cipher.
//
// Encryption is pinned only at padding 0. The reference fills the padding from
// `Math.random`, so with any other count its output is not a function of its
// inputs and no fixture could hold it. Zero padding removes the draw entirely
// and still exercises the whole cipher: key derivation, the marker walk, and
// the two rotations that follow every character.
//
// Decryption is deterministic at every padding count, so it is sampled across
// several -- including counts longer than the message, which yield the empty
// string rather than an error.
//
// The alphabet is 49 characters and nothing else is representable; inputs that
// leave it are rejected by the reference, so the samples stay inside it.

const PASSWORDS = ["", "a", "secret", "hello_world", "zzz", "0123456789", "-+*/"];
const MESSAGES = [
    "attack_at_dawn",
    "a",
    "",
    "the_quick_brown_fox",
    "0123456789",
    ",-+*/:?!'()",
    "_",
];
const SIGNATURES = ["", "alice", "bob_1"];

export function add({addCase}) {
    let index = 0;

    // Encryption, padding fixed at zero so the output is reproducible.
    for (const password of PASSWORDS) {
        for (const message of MESSAGES) {
            addCase(`ls47_enc_${index++}`, message, [
                {op: "LS47 Encrypt", args: [password, 0, ""]},
            ]);
        }
    }

    // The signature travels after a `---` separator that the cipher does not
    // treat specially, so it must round-trip like any other text.
    for (const signature of SIGNATURES) {
        addCase(`ls47_sig_${index++}`, "attack_at_dawn", [
            {op: "LS47 Encrypt", args: ["secret", 0, signature]},
        ]);
    }

    // Decryption across padding counts, including ones past the end.
    for (const padding of [0, 1, 5, 10, 100]) {
        addCase(`ls47_dec_${index++}`, "gpc.jyqvu-h'_kbxw", [
            {op: "LS47 Decrypt", args: ["secret", padding]},
        ]);
    }

    // Round trips: encrypt at zero padding, then decrypt at zero padding.
    for (const password of PASSWORDS.slice(0, 4)) {
        addCase(`ls47_round_${index++}`, "attack_at_dawn", [
            {op: "LS47 Encrypt", args: [password, 0, ""]},
            {op: "LS47 Decrypt", args: [password, 0]},
        ]);
    }
}
