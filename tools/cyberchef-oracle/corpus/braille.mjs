// Braille transcription and combining-mark decoration.

// The braille table is a position-for-position pairing, not an alphabet, so
// the risk is transcription order. These inputs walk the whole table rather
// than sampling it: every letter, every digit, and the punctuation whose
// ordering is North American Braille ASCII and therefore not derivable.
const BRAILLE_INPUTS = [
    "HELLO WORLD",
    "hello world",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "abcdefghijklmnopqrstuvwxyz",
    "0123456789",
    " '@/\"^>,*<-.%[$+!&;:\\(_?]#)=",
    "Mixed Case 123",
    "",
    " ",
    // Not in the table: passes through unchanged.
    "café 中文",
];

export async function add({addCase, bakeString}) {
    for (const [index, value] of BRAILLE_INPUTS.entries()) {
        addCase(`braille_encode_${index}`, value, [{op: "To Braille", args: []}]);
        // Round trip. Case does not survive — both cases map to one cell — so
        // this pins what actually comes back rather than assuming an inverse.
        const encoded = await bakeString(value, [{op: "To Braille", args: []}]);
        addCase(`braille_round_trip_${index}`, encoded, [{op: "From Braille", args: []}]);
    }

    // Decoding cells directly, including ones outside the six-dot range that
    // the table does not cover.
    for (const [index, value] of ["⠓⠑⠇⠇⠕", "⠀", "⠿", "⣿", "not braille"].entries()) {
        addCase(`braille_decode_${index}`, value, [{op: "From Braille", args: []}]);
    }

    // Unicode Text Format works on bytes, so a multi-byte character gets marks
    // inserted inside it. That is the reference's behaviour and is pinned
    // rather than tidied.
    for (const [index, value] of ["hello", "café", "", "a"].entries()) {
        const raw = Buffer.from(value, "utf8");
        for (const [underline, strike] of [
            [true, false],
            [false, true],
            [true, true],
            [false, false],
        ]) {
            addCase(
                `unicode_format_${index}_${underline ? 1 : 0}${strike ? 1 : 0}`,
                raw,
                [{op: "Unicode Text Format", args: [underline, strike]}],
            );
        }
    }
}
