// Rendering delimited text as a table.
//
// Two behaviours here look like bugs and are the operation, so both are
// pinned rather than described:
//
//   - The input is HTML-escaped *before* it is parsed. A quote has become
//     `&quot;` by the time the parser looks for one, so the quoted-field
//     handling can never fire and `"a,b",c` is three cells, not two. Asking to
//     split on `<` splits on nothing for the same reason.
//   - A row is recorded only once it holds a cell, so input carrying no
//     delimiter at all produces no rows and the operation returns the empty
//     string rather than a one-cell table.
//
// Markdown ignores the header flag entirely: the reference removes the first
// row whichever way the flag is set, because the renderer it targets will not
// display a table without a header. Both settings are sampled to hold that.

const INPUTS = [
    "a,b,c\r\nd,e,f",
    "name,age\r\nalice,30\r\nbob,4",
    // Ragged rows: the column widths come from the widest cell anywhere.
    "a,b,c\r\nd\r\ne,f",
    // No delimiter at all -- the empty-output case.
    "abcdef",
    // A trailing row delimiter, so the last row is closed by it.
    "a,b\r\n",
    // Empty cells on both sides of a delimiter.
    ",,\r\n,,",
    // Quotes, which the escaping has already turned into entities.
    '"a,b",c\r\nd,e',
    // Characters the escaper rewrites, which then pad to their escaped width.
    "&,<,>\r\n\",',`",
    // Non-ASCII, where the padding counts code units rather than bytes.
    "é,à\r\nb,c",
    // A single row.
    "one,two,three",
    // Rows of one cell each.
    "a\r\nb\r\nc",
];

const FORMATS = ["ASCII", "HTML", "Markdown"];

export function add({addCase}) {
    let index = 0;

    for (const input of INPUTS) {
        for (const format of FORMATS) {
            for (const header of [false, true]) {
                addCase(`table_${index++}`, input, [
                    {op: "To Table", args: [",", "\r\n", header, format]},
                ]);
            }
        }
    }

    // Every character of the delimiter argument is a delimiter in its own
    // right, so `,;` splits on either rather than on the pair.
    addCase(`table_multi_${index++}`, "a,b;c\r\nd,e;f", [
        {op: "To Table", args: [",;", "\r\n", false, "ASCII"]},
    ]);

    // Tab and pipe separated, the two the description names.
    addCase(`table_tsv_${index++}`, "a\tb\r\nc\td", [
        {op: "To Table", args: ["\t", "\r\n", true, "ASCII"]},
    ]);
    addCase(`table_psv_${index++}`, "a|b\r\nc|d", [
        {op: "To Table", args: ["|", "\r\n", true, "Markdown"]},
    ]);

    // A single row delimiter, so `\r\n` leaves an empty second cell rather
    // than being consumed as one ending.
    addCase(`table_lf_${index++}`, "a,b\r\nc,d", [
        {op: "To Table", args: [",", "\n", false, "ASCII"]},
    ]);

    // Two of the *same* row delimiter stay two endings, unlike a mixed pair.
    addCase(`table_blank_${index++}`, "a,b\n\nc,d", [
        {op: "To Table", args: [",", "\n", false, "ASCII"]},
    ]);

    // A delimiter the escaping removes, which therefore never matches.
    addCase(`table_escaped_delim_${index++}`, "a<b\r\nc<d", [
        {op: "To Table", args: ["<", "\r\n", false, "ASCII"]},
    ]);

    // A byte-order mark ahead of the first cell is dropped.
    addCase(`table_bom_${index++}`, "﻿a,b\r\nc,d", [
        {op: "To Table", args: [",", "\r\n", false, "ASCII"]},
    ]);
}
