// Case transforms, pinned across the inputs where two implementations of
// "upper case" stop agreeing.
//
// Random ASCII would pass trivially and prove nothing. The fixed inputs below
// carry a sharp s that grows into two characters when upper-cased, a final
// sigma whose lower-casing depends on its position in the word, a dotted
// capital I, a title-case digraph, and scripts with no case at all. Those are
// the places where Rust's Unicode mapping and JavaScript's could differ, so
// those are what get pinned.

const UNICODE_EDGES = [
    "HELLO ÄÖÜ ẞ ΣΟΦΟΣ İIİ ǅ АБВ 中文",
    "Hello ÄöÜ ß ẞ ΣσςΛ İi ǅ 123 中文 !?",
    "ΑΣ ΟΔΟΣ σοφος ΣΣΣ",
    "ǄǅǆǇǈǉǊǋǌ",
    "i̇ İ ı I",
];

const SCOPE_INPUT =
    "hello world. new sentence here\nsecond paragraph. third one\n  indented start";

export async function add({addCase, randomAscii}) {
    for (const [index, value] of UNICODE_EDGES.entries()) {
        addCase(`case_lower_edge_${index}`, value, [{op: "To Lower case", args: []}]);
        addCase(`case_upper_edge_${index}`, value, [{op: "To Upper case", args: ["All"]}]);
        addCase(`case_swap_edge_${index}`, value, [{op: "Swap case", args: []}]);
        addCase(`case_alternating_edge_${index}`, value, [{op: "Alternating Caps", args: []}]);
    }

    // Every capitalisation scope over the same input, so a divergence in one
    // scope cannot hide behind another passing.
    for (const scope of ["All", "Word", "Sentence", "Paragraph"]) {
        addCase(`case_upper_scope_${scope.toLowerCase()}`, SCOPE_INPUT, [
            {op: "To Upper case", args: [scope]},
        ]);
    }

    // Word scope decides boundaries with `\w`, which is ASCII in JavaScript
    // even under the Unicode flag. Underscores and digits are word characters;
    // hyphens and quotes are not.
    for (const value of ["a_b c-d e.f 1g h2 _x (y) 'z'", "_start", "9lives", "...", ""]) {
        addCase(`case_upper_word_${addCaseIndex(value)}`, value, [
            {op: "To Upper case", args: ["Word"]},
        ]);
    }

    for (const length of [0, 1, 7, 32]) {
        const raw = randomAscii(length).toString("latin1");
        addCase(`case_lower_${length}`, raw, [{op: "To Lower case", args: []}]);
        addCase(`case_swap_${length}`, raw, [{op: "Swap case", args: []}]);
        addCase(`case_alternating_${length}`, raw, [{op: "Alternating Caps", args: []}]);
    }

    // All casings is exponential, so the pinned inputs stay short by
    // necessity. The empty and single-character cases are the ones most likely
    // to be got wrong at the edges of the loop.
    for (const value of ["", "z", "aB1", "a-b"]) {
        addCase(`case_all_casings_${addCaseIndex(value)}`, value, [
            {op: "Get All Casings", args: []},
        ]);
    }
}

/// A stable, filesystem-safe suffix for a fixed input.
function addCaseIndex(value) {
    if (value === "") return "empty";
    return value.replace(/[^a-z0-9]/gi, "_").toLowerCase();
}
