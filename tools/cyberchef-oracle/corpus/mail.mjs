// Quoted-printable, PEM, NT hashes, and Citrix CTX1.
//
// Four formats that carry credentials or certificates around, grouped because
// each one's interesting cases are about text encoding rather than about the
// algorithm: which bytes stay literal, where a line breaks, and whether the
// input is read as UTF-8 or UTF-16.

/** Payloads chosen for where quoted-printable's line breaking has to decide. */
const QP_INPUTS = [
    "",
    "hello",
    // Every byte, so each range boundary in the escape table is crossed.
    Buffer.from(Array.from({length: 256}, (_, index) => index)),
    // The characters that look printable and are escaped anyway.
    '"?_=',
    // Trailing blanks, which are escaped so a mail agent cannot strip them.
    "line with trailing spaces   ",
    "tab\tat\tend\t",
    "trailing space before break   \nnext line",
    // Line endings of all three shapes, normalised to CRLF.
    "a\nb\r\nc\rd",
    // Exactly at, just under, and just over the 76-character limit.
    "x".repeat(75),
    "x".repeat(76),
    "x".repeat(77),
    "x".repeat(100),
    "x".repeat(200),
    // A long run of escapes, where a break must not split `=XX` in half.
    "é".repeat(40),
    // Words, so the break lands on a separator rather than mid-word.
    "The quick brown fox jumps over the lazy dog. ".repeat(4),
    // A multi-byte character right at the boundary, which the encoder moves
    // whole rather than splitting across the break.
    `${"x".repeat(70)}ééé`,
    `${"x".repeat(72)}€`,
    `${"x".repeat(74)}€`,
    "a".repeat(60) + " " + "b".repeat(30),
];

/** Hex bodies and labels for PEM. */
const PEM_CASES = [
    ["2a864886f70d01010b", "CERTIFICATE"],
    ["", "CERTIFICATE"],
    ["aabb", "MY KEY"],
    ["00", "PUBLIC KEY"],
    // A body that folds at exactly sixty-four base64 characters, where the
    // trailing-whitespace trim decides whether a blank line appears.
    ["ab".repeat(48), "CERTIFICATE"],
    ["ab".repeat(47), "CERTIFICATE"],
    ["ab".repeat(49), "CERTIFICATE"],
    ["ab".repeat(200), "CERTIFICATE"],
    // Whitespace in the input hex, which is stripped first.
    ["2a 86 48\n86f7", "CERTIFICATE"],
];

/** PEM text fed straight to the reader, including shapes no writer produces. */
const PEM_TEXT = [
    "-----BEGIN CERTIFICATE-----\r\nKoZIhvcNAQEL\r\n-----END CERTIFICATE-----\r\n",
    "-----BEGIN CERTIFICATE-----\nKoZIhvcNAQEL\n-----END CERTIFICATE-----\n",
    // Two blocks in one file, which is what a certificate chain looks like.
    "-----BEGIN CERTIFICATE-----\nqrs=\n-----END CERTIFICATE-----\n" +
        "-----BEGIN CERTIFICATE-----\nAAE=\n-----END CERTIFICATE-----\n",
    // Commentary around and between blocks, which is ignored.
    "subject: example\n-----BEGIN CERTIFICATE-----\nqrs=\n-----END CERTIFICATE-----\ntrailing\n",
    // No block at all.
    "nothing here",
    "",
    // A lower-case label, which is not a header.
    "-----BEGIN certificate-----\nqrs=\n-----END certificate-----\n",
    // A label with a space in it.
    "-----BEGIN MY KEY-----\nqrs=\n-----END MY KEY-----\n",
];

/** Passwords, chosen for the UTF-16 question rather than for length. */
const PASSWORDS = [
    "",
    "a",
    "password",
    "Password1!",
    // Non-ASCII, where hashing UTF-8 instead of UTF-16LE would diverge.
    "pässword",
    "你好",
    "éèê",
    " leading and trailing ",
    "x".repeat(64),
];

export async function add({addCase, bakeString}) {
    for (const [index, payload] of QP_INPUTS.entries()) {
        const input = Buffer.isBuffer(payload) ? payload : Buffer.from(payload, "utf8");
        // One recipe rather than two cases: the corpus checks every prefix, so
        // this pins the encoding and that it reads back unchanged.
        addCase(`qp_round_trip_${index}`, input, [
            {op: "To Quoted Printable", args: []},
            {op: "From Quoted Printable", args: []},
        ]);
    }

    for (const [index, [hex, label]] of PEM_CASES.entries()) {
        addCase(`pem_write_${index}`, hex, [{op: "Hex to PEM", args: [label]}]);
        const pem = await bakeString(hex, [{op: "Hex to PEM", args: [label]}]);
        addCase(`pem_round_trip_${index}`, pem, [{op: "PEM to Hex", args: []}]);
    }
    for (const [index, text] of PEM_TEXT.entries()) {
        addCase(`pem_read_${index}`, text, [{op: "PEM to Hex", args: []}]);
    }

    for (const [index, password] of PASSWORDS.entries()) {
        addCase(`nt_hash_${index}`, password, [{op: "NT Hash", args: []}]);
        addCase(`ctx1_round_trip_${index}`, password, [
            {op: "Citrix CTX1 Encode", args: []},
            {op: "Citrix CTX1 Decode", args: []},
        ]);
    }
}
