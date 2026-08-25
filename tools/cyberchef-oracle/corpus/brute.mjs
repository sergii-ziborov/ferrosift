// Rotation brute force.
//
// Two details decide whether a port of these is right, and neither shows up
// on a plain ASCII sentence with default arguments. The amount column is
// right-aligned in two characters by `(" " + amount).slice(-2)`, so shifts
// below ten carry a leading space. And the output runs through
// `escapeWhitespace`, whose character range is written `\x09-\x10` — tab
// through data-link escape, which is almost certainly not what was meant but
// is what the bytes say. Inputs below carry control characters so that range
// is exercised rather than assumed.

const SAMPLES = [
    "Uryyb, Jbeyq!",
    "The quick brown fox jumps over the lazy dog",
    "MIXED case 123 and !@# symbols",
    "with\ttab and\nnewline",
    // Control characters either side of the `\x09-\x10` range the reference
    // escapes: bell and backspace below it, data-link escape at its top, and
    // device-control-one just past it.
    "a\x07b\x08c\x09d\x10e\x11f",
    "",
    "a",
];

export async function add({addCase, randomAscii}) {
    for (const [index, value] of SAMPLES.entries()) {
        const raw = Buffer.from(value, "latin1");

        // Defaults: every shift printed, with the amount column.
        addCase(`rot13_brute_${index}`, raw, [
            {op: "ROT13 Brute Force", args: [true, true, false, 100, 0, true, ""]},
        ]);
        addCase(`rot47_brute_${index}`, raw, [
            {op: "ROT47 Brute Force", args: [100, 0, true, ""]},
        ]);

        // Without the amount column, so the alignment code is not the only
        // thing being compared.
        addCase(`rot13_brute_bare_${index}`, raw, [
            {op: "ROT13 Brute Force", args: [true, true, false, 100, 0, false, ""]},
        ]);

        // Numbers rotated, and each case class on its own.
        addCase(`rot13_brute_numbers_${index}`, raw, [
            {op: "ROT13 Brute Force", args: [true, true, true, 100, 0, true, ""]},
        ]);
        addCase(`rot13_brute_lower_only_${index}`, raw, [
            {op: "ROT13 Brute Force", args: [true, false, false, 100, 0, true, ""]},
        ]);
        addCase(`rot13_brute_upper_only_${index}`, raw, [
            {op: "ROT13 Brute Force", args: [false, true, false, 100, 0, true, ""]},
        ]);
    }

    // A crib that matches one shift, one that matches none, and one whose case
    // differs from the text it must match.
    const cipher = Buffer.from("Uryyb, Jbeyq!", "latin1");
    for (const crib of ["hello", "HELLO", "zzzznotpresent", "world"]) {
        addCase(`rot13_brute_crib_${crib}`, cipher, [
            {op: "ROT13 Brute Force", args: [true, true, false, 100, 0, true, crib]},
        ]);
    }

    // Sample windows: shorter than the input, offset into it, and both
    // running past the end where the reference's slice clamps.
    for (const [length, offset] of [[5, 0], [5, 3], [100, 40], [3, 100], [0, 0]]) {
        addCase(`rot13_brute_window_${length}_${offset}`, cipher, [
            {op: "ROT13 Brute Force", args: [true, true, false, length, offset, true, ""]},
        ]);
        addCase(`rot47_brute_window_${length}_${offset}`, cipher, [
            {op: "ROT47 Brute Force", args: [length, offset, true, ""]},
        ]);
    }

    for (const length of [0, 8, 32]) {
        addCase(`rot47_brute_random_${length}`, randomAscii(length), [
            {op: "ROT47 Brute Force", args: [100, 0, true, ""]},
        ]);
    }
}
