// Caret/M decoding, case-insensitive regex folding, and power sets.

// The caret decoder is a state machine, so the cases that matter are the ones
// that leave it mid-sequence or push it just past a range boundary. The
// reference never flushes a pending state at the end of input, so a trailing
// "M", "M-", "M-^", or "^" vanishes — pinned here so the port cannot quietly
// decide to emit it instead.
const CARET_INPUTS = [
    "^A^B^C",
    "M-AM-BM-C",
    "M-^AM-^B",
    // Boundaries: 63 is the special case in three of the four branches.
    "^?",
    "M-^?",
    "^>",
    "^@",
    "^~",
    "M- ",
    "M-~",
    // Sequences that do not complete: the letter is emitted literally.
    "M!",
    "M-\\x01",
    "^\\x01",
    // Pending at end of input, which the reference drops.
    "abcM",
    "abcM-",
    "abcM-^",
    "abc^",
    "",
    "plain text with no markers",
    "mixed ^A and M-B and M-^C together",
];

const REGEX_INPUTS = [
    "[aA][bB][cC]",
    "[aA]",
    // Not a case fold: two different letters stay as a class.
    "[ab]",
    "[Ab][aB]",
    // Not four characters, or not letters.
    "[a]",
    "[abc]",
    "[a1]",
    "[]",
    "no brackets here",
    "",
    "trailing [aA",
    "[aA][ab][bB]",
];

const POWER_SET_INPUTS = [
    "a,b,c",
    "a,b",
    "a",
    "",
    // Empty items are filtered before the subsets are built.
    "a,,b",
    ",,,",
    "one,two,three,four",
];

export async function add({addCase}) {
    for (const [index, value] of CARET_INPUTS.entries()) {
        addCase(`caret_m_decode_${index}`, value, [{op: "Caret/M-decode", args: []}]);
    }

    for (const [index, value] of REGEX_INPUTS.entries()) {
        addCase(`regex_case_fold_${index}`, value, [
            {op: "From Case Insensitive Regex", args: []},
        ]);
    }

    for (const [index, value] of POWER_SET_INPUTS.entries()) {
        addCase(`power_set_${index}`, value, [{op: "Power Set", args: [","]}]);
    }

    // A different delimiter, including a multi-character one, since the same
    // token is used both to split the input and to join each subset.
    addCase("power_set_space", "a b c", [{op: "Power Set", args: [" "]}]);
    addCase("power_set_multi", "a::b::c", [{op: "Power Set", args: ["::"]}]);
    addCase("power_set_dash", "x-y-z", [{op: "Power Set", args: ["-"]}]);
}
