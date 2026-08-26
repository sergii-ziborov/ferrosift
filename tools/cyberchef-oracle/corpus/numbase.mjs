// Reading and writing a number in another base.
//
// The two are not inverses, and the corpus is built to show it. `To Base`
// hands its value to the reference's `toString(base)`. `From Base` does not
// hand its text to the matching constructor: it splits on the point itself and
// reads each fractional digit alone. So a value whose letters mix case is read
// by one and refused by the other, and a fraction rounds once per digit here
// where the constructor rounds once for the whole.
//
// Both ends of the value pipeline are exercised, because these are the first
// two operations where a number arrives *and* leaves: `To Base` takes a
// BigNumber from the dish and `From Base` hands one back.

const RADICES = [2, 8, 10, 16, 36, 3, 7, 12, 20, 26, 35];

// Values for `To Base`, given to the recipe as text for the dish to read.
const NUMBERS = [
    "255", "0", "1", "-255", "-1", "35", "36", "1295",
    // Past 2^53, where a float has stopped counting by ones.
    "9007199254740993", "123456789012345678901234567890",
    // Fractions that terminate in some bases and not in others.
    "0.5", "0.1", "0.25", "255.5", "-0.75",
    // Small enough to round away entirely, and large enough to be long.
    "1e-30", "1e-8", "1e21", "1e30",
    // The epoch constant, which the filetime operations render in hexadecimal.
    "116444736000000000",
    // The specials, which render rather than refuse.
    "NaN", "Infinity", "-Infinity",
    // Text the dish cannot read becomes not-a-number rather than an error,
    // which is the difference between a dish and a direct constructor call.
    "apples", "",
];

// Text for `From Base`, paired with the base to read it in.
const WRITTEN = [
    // The alphabet, in either case.
    ["ff", 16], ["FF", 16], ["zz", 36], ["ZZ", 36],
    ["1010", 2], ["777", 8], ["123", 10], ["hello", 36],
    // Signs. A mixed-case value, a `0x` prefix and a digit outside the base
    // all make the reference *throw*, so they cannot be pinned as output here
    // -- tests/conformance_numbase.rs holds them.
    ["-ff", 16], ["+ff", 16],
    // `e` as a digit rather than as an exponent marker.
    ["1e5", 16], ["1e5", 15],
    // Fractions: the digits after the point are read one at a time, so mixed
    // case is accepted there and refused before it.
    ["ff.8", 16], ["1F.A", 16], ["1F.aB", 16], ["1f.Ab", 16],
    ["0.8", 16], [".8", 16], ["8.", 16],
    ["1010.1011", 2], ["0.1", 3], ["0.0001", 2], ["0.zzzzzzzz", 36],
    // A second point, and everything after it, is dropped: the reference reads
    // the first two pieces and looks no further.
    ["1.2.3", 16], ["1.8.ff", 16],
    // Whitespace is removed from everywhere, not only from the ends.
    [" ff ", 16], ["f f", 16], ["1 0 1 0", 2], ["f\tf\nf", 16],
    // Nothing at all, which is zero with a base and refused without one.
    ["", 16], ["   ", 16], [".", 16],
    // Far past any fixed width.
    ["ffffffffffffffffffffffffffffffff", 16],
    ["19db1ded53e8000", 16],
];

export async function add({addCase}) {
    for (const [index, number] of NUMBERS.entries()) {
        for (const radix of [16, 2, 36]) {
            addCase(`tobase_${radix}_${index}`, number, [{op: "To Base", args: [radix]}]);
        }
    }
    // Every base on one value, so the alphabet is pinned end to end rather
    // than only where a sample happened to land.
    for (let radix = 2; radix <= 36; radix += 1) {
        addCase(`tobase_alphabet_${radix}`, "123456789", [{op: "To Base", args: [radix]}]);
        addCase(`tobase_alphabet_frac_${radix}`, "0.1", [{op: "To Base", args: [radix]}]);
    }

    for (const [index, [text, radix]] of WRITTEN.entries()) {
        addCase(`frombase_${index}`, text, [{op: "From Base", args: [radix]}]);
    }
    for (const radix of RADICES) {
        addCase(`frombase_radix_${radix}`, "1010", [{op: "From Base", args: [radix]}]);
    }

    // A number carried between the two, which is where the value model does
    // its work: `From Base` hands back a BigNumber and the dish renders it
    // with `toFixed` for whatever comes next.
    addCase("numbase_round_trip", "ff", [
        {op: "From Base", args: [16]},
        {op: "To Base", args: [16]},
    ]);
    addCase("numbase_round_trip_fraction", "ff.8", [
        {op: "From Base", args: [16]},
        {op: "To Base", args: [2]},
    ]);
    addCase("numbase_into_sum", "ff", [
        {op: "From Base", args: [16]},
        {op: "Sum", args: ["Space"]},
    ]);
    addCase("numbase_from_sum", "1 2 3", [
        {op: "Sum", args: ["Space"]},
        {op: "To Base", args: [2]},
    ]);
}
