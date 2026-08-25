// Pass-through operations, and counting.

// The three pass-throughs are pinned rather than assumed. "It returns its
// input" is exactly the kind of claim that is true until someone adds a trim,
// and a recipe with a comment in the middle of it should produce the same
// bytes as one without.
const PASSTHROUGH_INPUTS = [
    "hello world",
    "<p>markup</p>",
    "  leading and trailing  ",
    "line\nbreaks\n",
    "",
    "café 中文 😀",
];

const COUNT_CASES = [
    ["hello world hello", "hello", "Simple string"],
    ["hello world hello", "l", "Simple string"],
    ["aaaa", "aa", "Simple string"],
    ["hello", "z", "Simple string"],
    ["hello", "", "Simple string"],
    ["", "x", "Simple string"],
    // Extended reads escape sequences before searching.
    ["a\tb\tc", "\\t", "Extended (\\n, \\t, \\x...)"],
    ["a\nb", "\\n", "Extended (\\n, \\t, \\x...)"],
    ["aAbB", "\\x41", "Extended (\\n, \\t, \\x...)"],
    // Regex is case-insensitive, and an unparsable pattern counts nothing.
    ["Hello hello HELLO", "hello", "Regex"],
    ["a1b2c3", "[0-9]", "Regex"],
    ["aaa", "a+", "Regex"],
    ["abc", "(", "Regex"],
    ["abc", "z+", "Regex"],
];

export async function add({addCase}) {
    for (const [index, value] of PASSTHROUGH_INPUTS.entries()) {
        addCase(`html_to_text_${index}`, value, [{op: "HTML To Text", args: []}]);
    }

    for (const [index, [text, search, type]] of COUNT_CASES.entries()) {
        addCase(`count_${index}`, text, [
            {op: "Count occurrences", args: [{option: type, string: search}]},
        ]);
    }
}
