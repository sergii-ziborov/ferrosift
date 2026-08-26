// Reading one colour notation and reporting it in all of them.
//
// The output carries the reference's colour-picker markup verbatim, so this
// is pinned as markup rather than as the stripped text.
//
// Two behaviours are pinned because they look like errors and are not:
//
//   - Nothing recognised is not a failure. The channels stay at their opening
//     values and the operation reports black at full alpha, because the
//     reference has no else-branch. An unparseable input and a literal black
//     give the same answer.
//   - A fully black colour divides by zero three times over while computing
//     CMYK. The reference tests the *result* rather than the divisor and
//     prints a bare `0`, which is not the two-decimal form every other ink
//     gets -- so `cmyk(0, 0, 0, 1.00)` has one field written differently from
//     its neighbours.
//
// The hex pattern is unanchored, so a colour inside a longer string is found.

const INPUTS = [
    "#d9edf7",
    "#000000",
    "#ffffff",
    "#FF8800",
    // Unanchored: found inside surrounding text.
    "background: #123456; border: 1px",
    // More digits than six -- only the first six are read.
    "#deadbeef",
    "rgb(217,237,247)",
    "rgb(217, 237, 247)",
    "rgba(217, 237, 247, 1)",
    "rgba(0, 0, 0, 0.5)",
    // Fractional channels survive into the rgb output but round for hex.
    "rgb(1.5, 2.5, 3.5)",
    "hsl(200, 65%, 91%)",
    "hsla(200, 65%, 91%, 1)",
    // Zero saturation takes the achromatic branch.
    "hsl(0, 0%, 50%)",
    "hsl(120, 100%, 50%)",
    "hsl(240, 100%, 25%)",
    "cmyk(0.12, 0.04, 0.00, 0.03)",
    // Full black, where three divisions have no answer.
    "cmyk(0, 0, 0, 1)",
    "cmyk(0, 0, 0, 0)",
    // Nothing recognised.
    "not a colour at all",
    "",
    // Each channel maximal in turn, which picks a different hue branch.
    "rgb(255, 0, 0)",
    "rgb(0, 255, 0)",
    "rgb(0, 0, 255)",
    // Green below blue, which adds a full turn to the hue.
    "rgb(255, 10, 200)",
    // Every channel equal, the achromatic case on the way back.
    "rgb(128, 128, 128)",
];

export function add({addCase}) {
    let index = 0;
    for (const input of INPUTS) {
        addCase(`colour_${index++}`, input, [{op: "Parse colour code", args: []}]);
    }
}