// ANSI stripping, HTTP framing, wrapping, and alphabet ranges.
//
// Each of these was ported from a regular expression or from string index
// arithmetic, and in both cases the interesting behaviour is at the edges the
// happy path never reaches: a control sequence that runs off the end of the
// input, a response with no blank line at all, a chunk size line carrying
// extensions, a wrap width that lands exactly on a boundary. Those are what
// the cases below pin.

const ANSI = [
    "[31mred[0m plain",
    "[1;32;40mbold green[m",
    "no escapes at all",
    // Runs off the end: the reference's regex does not match, so these stay.
    "trailing [",
    "trailing [31",
    "",
    "[?25l hidden cursor [?25h",
    // Intermediate bytes between parameters and the final byte.
    "[0 qcursor",
    // An escape that is not followed by a bracket is not a sequence.
    "Xnot a csi",
];

const HTTP_RESPONSES = [
    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nbody here",
    "HTTP/1.1 200 OK\nContent-Type: text/plain\n\nbody here",
    // No blank line anywhere: the reference returns the input untouched.
    "HTTP/1.1 200 OK\r\nContent-Type: text/plain",
    "no headers at all",
    "",
    // A blank line first, so the body is everything after it.
    "\r\n\r\nbody",
];

const CHUNKED = [
    "4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n",
    "4\nWiki\n5\npedia\n0\n\n",
    // Chunk extensions after the size: parseInt takes the leading hex only.
    "4;name=value\r\nWiki\r\n0\r\n\r\n",
    // Upper-case hex digits.
    "A\r\n0123456789\r\n0\r\n\r\n",
    // No terminating zero chunk: the loop ends when a size will not parse.
    "4\r\nWiki\r\n",
    // A size line that is not hex at all ends the body immediately.
    "zz\r\nWiki\r\n",
    "",
];

const RANGES = ["a-z", "a-f0-9", "A-Za-z0-9+/=", "abc", "", "\\-", "a-c-e"];

export async function add({addCase, randomAscii}) {
    for (const [index, value] of ANSI.entries()) {
        addCase(`ansi_strip_${index}`, value, [{op: "Remove ANSI Escape Codes", args: []}]);
    }

    for (const [index, value] of HTTP_RESPONSES.entries()) {
        addCase(`http_strip_headers_${index}`, value, [{op: "Strip HTTP headers", args: []}]);
    }

    for (const [index, value] of CHUNKED.entries()) {
        addCase(`http_dechunk_${index}`, value, [{op: "Dechunk HTTP response", args: []}]);
    }

    // Wrap: widths on, either side of, and far from a boundary, plus input
    // that already contains line feeds — which the reference's dot does not
    // match, so they are dropped rather than counted.
    for (const width of [1, 4, 8, 64]) {
        for (const length of [0, 1, 7, 8, 9, 40]) {
            addCase(
                `wrap_${width}_${length}`,
                randomAscii(length).toString("latin1"),
                [{op: "Wrap", args: [width]}],
            );
        }
        addCase(`wrap_multiline_${width}`, "first line\nsecond line\n\nfourth", [
            {op: "Wrap", args: [width]},
        ]);
    }

    for (const [index, value] of RANGES.entries()) {
        addCase(`alphabet_expand_${index}`, value, [
            {op: "Expand alphabet range", args: [""]},
        ]);
        addCase(`alphabet_expand_delim_${index}`, value, [
            {op: "Expand alphabet range", args: [","]},
        ]);
    }
}
