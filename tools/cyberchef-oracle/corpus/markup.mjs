// Tag stripping and smart-character folding.

// The smart map has entries that expand to more than one character, entries
// that collapse several sources to one target, and five different kinds of
// non-ASCII space. Any of those is easy to transcribe wrong, so the inputs
// below carry them rather than a representative sample.
const SMART_INPUTS = [
    "“double” and ‘single’ quotes",
    "en – dash, em — dash, hyphen ‐",
    "ellipsis… and © ® ™",
    "arrows ← → ↑ ↓ ↔ ⇐ ⇒ ⇔",
    "guillemets « » ‹ ›",
    "maths × ÷ ± • ·",
    "spaces:     :end",
    "plain ASCII only",
    "",
    // Not in the table, so the unmappable argument decides.
    "unmapped 中文 and é",
    "“mixed” 中 — é",
];

const HTML_INPUTS = [
    "<p>Hello <b>world</b></p>",
    "<div>\n    indented\n    lines\n</div>",
    "<p>one</p>\n\n\n<p>two</p>",
    // Nested angle brackets: one pass would leave a stray bracket behind,
    // which is why the reference removes recursively.
    "a<<b>>c",
    "<<nested>>",
    // Not a tag: `<[^>]+>` needs at least one character inside.
    "a<>b",
    "less < than and greater > than",
    "no markup at all",
    "",
    "\n\n\nleading blank lines",
    "trailing blank lines\n\n\n",
];

export async function add({addCase}) {
    for (const [index, value] of SMART_INPUTS.entries()) {
        for (const unmappable of ["Include", "Remove", "Replace with '.'"]) {
            const label = unmappable === "Include" ? "inc" : unmappable === "Remove" ? "rem" : "dot";
            addCase(`smart_${label}_${index}`, value, [
                {op: "Escape Smart Characters", args: [unmappable]},
            ]);
        }
    }

    for (const [index, value] of HTML_INPUTS.entries()) {
        // All four combinations of the two tidying flags, because each one
        // rewrites whitespace the other may have created.
        for (const indent of [true, false]) {
            for (const breaks of [true, false]) {
                addCase(`html_strip_${index}_${indent ? 1 : 0}${breaks ? 1 : 0}`, value, [
                    {op: "Strip HTML tags", args: [indent, breaks]},
                ]);
            }
        }
    }
}
