// Widening a regular expression to match either case.
//
// The operation is nine sequential global replacements over each other's
// output, so a case that only exercises one of them proves very little. Each
// sample below is chosen from the reference's own worked examples, one per
// pass, plus the documented end-to-end example.
//
// `\!-D` is here because of an index slide, not despite it: the reference
// reads the match's third character, and when an optional backslash is
// consumed that third character is the hyphen rather than the range's high
// end. The output is odd and it is what the reference produces.

const SAMPLES = [
    "Mozilla/[0-9].[0-9] .*",
    "[test]",
    "[A-Z]",
    "[a-z]",
    "[H-d]",
    "[!-D]",
    "[%-^]",
    "[K-`]",
    "[[-}]",
    "[b-}]",
    "[<-j]",
    "[^-j]",
    "\\!-D",
    "[A-Za-z0-9]",
    "plain text, no ranges",
    "",
];

export function add({addCase}) {
    for (const [index, sample] of SAMPLES.entries()) {
        addCase(`case_widen_${index}`, sample, [
            {op: "To Case Insensitive Regex", args: []},
        ]);
    }
}
