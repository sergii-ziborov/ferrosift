// Comparing samples position by position.
//
// This is the first family whose output is markup. What is pinned is the
// markup itself, which the harness reads from the dish's own value -- every
// `get` translates by way of an ArrayBuffer, and the HTML dish's conversion to
// one strips the tags, so asking for bytes returns the highlighting with the
// highlighting taken out. Pinned that way, a port that emitted no spans at all
// would have passed.
//
// The chained cases at the end are the ones that prove the value model rather
// than the operation. A step *after* a markup operation receives the stripped
// and unescaped text, not the markup -- so `Offset checker` into `To Upper
// case` has no `SPAN` in it. Until markup was its own kind, FerroSift passed
// the tags on and the harness had to refuse to pin such a recipe at all.
//
// The highlighting is driven by a single `inMatch` flag that is mutated inside
// the loop over samples but only updated while writing the *last* one. Earlier
// samples in a row therefore act on a run the last one has not opened yet,
// which is where the stray closing tag at the end of a run comes from. Samples
// of differing lengths make it visible, so most cases below have them.
//
// The escaping is the reference's own table and is not the usual one: `'`
// becomes `&#x27;` rather than `&apos;`, a backtick is escaped as well, and a
// NUL becomes a private-use character instead of an entity.

const SAMPLES = [
    // Identical samples: one run covering everything.
    "abcdef\n\nabcdef",
    // A single difference in the middle breaks the run in two.
    "abcdef\n\nabcXef",
    // Nothing in common at all.
    "abcdef\n\nuvwxyz",
    // Differing lengths, shorter second.
    "abcdef\n\nabc",
    // Differing lengths, shorter first -- the tail of the longer sample is
    // emitted while walking the first sample's last position.
    "abc\n\nabcdef",
    // Three samples, so the flag is written on the third and the earlier two
    // act on it stale.
    "abcdef\n\nabcdef\n\nabcdef",
    "abcdef\n\nabcXef\n\nabcdef",
    "abcdef\n\nabc\n\nabcdefgh",
    // A match that runs to the very end of a sample.
    "ab\n\nab",
    // A match that ends exactly where the shorter sample does.
    "abcd\n\nab",
    // One character each.
    "a\n\na",
    "a\n\nb",
    // Empty samples on either side.
    "\n\nabc",
    "abc\n\n",
    "\n\n",
    // Every character the escaper touches, matching and not.
    "&<>\"'`\n\n&<>\"'`",
    "&<>\"'`\n\nxxxxxx",
    // Non-ASCII inside and outside a run.
    "aébc\n\naébc",
    "aébc\n\naXbc",
];

// Delimiters are given as the characters themselves rather than as escape
// sequences: the bake harness hands an argument through unchanged, so a
// written `\n` would reach the operation as a backslash and an `n`.
const DELIMITERS = ["\n\n", ",", "\t", "||"];

export function add({addCase}) {
    let index = 0;
    for (const sample of SAMPLES) {
        addCase(`offset_check_${index++}`, sample, [
            {op: "Offset checker", args: ["\n\n"]},
        ]);
    }
    // A different delimiter is joined back with itself, escaped, so the
    // separator in the output is not always the one that was typed.
    for (const delimiter of DELIMITERS) {
        addCase(`offset_delim_${index++}`, `abcdef${delimiter}abcXef`, [
            {op: "Offset checker", args: [delimiter]},
        ]);
    }

    // Chained past the markup operation: the next step must receive the
    // stripped, unescaped text rather than the tags. This is what the value
    // model buys, and what the harness previously refused to pin.
    for (const sample of ["abcdef\n\nabcXef", "&<>\"'`\n\n&<>\"'`", "abc\n\nabcdef"]) {
        addCase(`offset_chain_upper_${index++}`, sample, [
            {op: "Offset checker", args: ["\n\n"]},
            {op: "To Upper case", args: ["All"]},
        ]);
        addCase(`offset_chain_hex_${index++}`, sample, [
            {op: "Offset checker", args: ["\n\n"]},
            {op: "To Hex", args: ["Space", 0]},
        ]);
    }
}