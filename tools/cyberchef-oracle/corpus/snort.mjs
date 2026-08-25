// Base92 and the SNORT hex-content notation.
//
// Base92 is sampled over text rather than bytes because the reference reads a
// string and takes `charCodeAt` of each character. The non-ASCII cases are the
// point of that: a code unit above 255 contributes more than eight bits to the
// stream and shifts every symbol after it, so an implementation that walks
// bytes instead of UTF-16 code units passes the ASCII cases and fails these.
//
// Hex content is sampled across all three conversion modes with spacing both
// on and off, because the modes differ only on the space byte and the spacing
// flag only inside a hex run — neither is visible in a recipe that avoids one.

const BASE92_TEXT = [
    "",
    "a",
    "ab",
    "hello world",
    "The quick brown fox",
    "éèü",
    "你好",
    "mixed é ascii",
];

const HEX_CONTENT_TEXT = [
    "foo=bar",
    "plain",
    "|3d|",
    "|3d 3e|",
    "a|3d3|b",
    "a|3 d3|b",
    "|  |",
    "||",
    "trailing|",
    "|41 42 43|xyz",
];

export function add({addCase, randomBytes}) {
    for (const [index, text] of BASE92_TEXT.entries()) {
        addCase(`base92_encode_${index}`, text, [{op: "To Base92", args: []}]);
    }
    // Decode is pinned on symbols the encoder itself produces, plus a lone
    // trailing symbol, which is the six-bit tail the pair form never reaches.
    for (const [index, encoded] of ["", "!", "AB", "ABC", "ABCD", "}}", "!!!"].entries()) {
        addCase(`base92_decode_${index}`, encoded, [{op: "From Base92", args: []}]);
    }

    for (const length of [0, 1, 8, 32]) {
        const raw = randomBytes(length);
        for (const convert of [
            "Only special chars",
            "Only special chars including spaces",
            "All chars",
        ]) {
            for (const spaces of [false, true]) {
                const tag = `${convert.split(" ")[1]}_${spaces ? "sp" : "nosp"}`;
                addCase(`hexcontent_encode_${tag}_${length}`, raw, [
                    {op: "To Hex Content", args: [convert, spaces]},
                ]);
            }
        }
    }
    // A printable sample exercises the literal path, which random bytes reach
    // only by accident.
    for (const convert of ["Only special chars", "Only special chars including spaces"]) {
        addCase(`hexcontent_encode_text_${convert.split(" ")[1]}`, Buffer.from("foo=bar baz!"), [
            {op: "To Hex Content", args: [convert, false]},
        ]);
    }

    for (const [index, text] of HEX_CONTENT_TEXT.entries()) {
        addCase(`hexcontent_decode_${index}`, text, [{op: "From Hex Content", args: []}]);
    }
}
