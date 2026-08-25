// Substitution, string unescaping, and De Bruijn sequences.

// Substitute reads both alphabets through the same range expansion the
// encodings use, so these cases exercise that path a second way — including
// the mismatched-length case, where the reference emits a warning line and
// carries on rather than refusing.
const SUBSTITUTE_CASES = [
    ["Hello, World!", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "XYZABCDEFGHIJKLMNOPQRSTUVW", false],
    ["Hello, World!", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "XYZABCDEFGHIJKLMNOPQRSTUVW", true],
    // Range expressions on both sides.
    ["hello world", "a-z", "n-za-m", false],
    ["HELLO world", "a-z", "n-za-m", true],
    // Mismatched lengths: warning, then as many pairs as the shorter allows.
    ["abcdef", "abc", "xyzw", false],
    ["abcdef", "abcd", "xy", false],
    // A plaintext symbol listed twice; the later mapping is the one that wins.
    ["aaa", "aa", "xy", false],
    // Symbols outside the alphabet pass through untouched.
    ["a1b2c3 !@#", "abc", "xyz", false],
    ["", "abc", "xyz", false],
    // Case-insensitive with an uncased character in the input.
    ["aA1 中", "a", "z", true],
];

const UNESCAPE_INPUTS = [
    "line\\nbreak",
    "tab\\there",
    "hex \\x41\\x42",
    "unicode \\u0041\\u00e9",
    "backslash \\\\ and quote \\'",
    "octal \\101",
    "not an escape \\q",
    "trailing backslash \\",
    "",
    "plain text",
    "\\r\\n\\t\\0",
];

export async function add({addCase}) {
    for (const [index, [text, plain, cipher, ignoreCase]] of SUBSTITUTE_CASES.entries()) {
        addCase(`substitute_${index}`, text, [
            {op: "Substitute", args: [plain, cipher, ignoreCase]},
        ]);
    }

    for (const [index, value] of UNESCAPE_INPUTS.entries()) {
        addCase(`unescape_string_${index}`, value, [{op: "Unescape string", args: []}]);
    }

    // De Bruijn ignores its input, so the arguments are what vary. An empty
    // input is the natural way to call a generator, and it is exactly the case
    // the expansion-ratio check used to refuse — so it is what gets pinned.
    for (const [k, n] of [[2, 2], [2, 3], [2, 8], [3, 2], [3, 4], [5, 3], [9, 2], [9, 4]]) {
        addCase(`de_bruijn_${k}_${n}`, "", [
            {op: "Generate De Bruijn Sequence", args: [k, n]},
        ]);
    }
    // A non-empty input must not change the answer.
    addCase("de_bruijn_input_ignored", "this text is discarded", [
        {op: "Generate De Bruijn Sequence", args: [3, 3]},
    ]);
}
