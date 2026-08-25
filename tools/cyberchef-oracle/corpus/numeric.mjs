// IEEE-754 packing and delimited-list reshaping.
//
// The float half is really a test of how JavaScript spells a number: the bytes
// are standard, and everything interesting happens when `To Float` turns a
// double back into text. The list half is a test of two things that look like
// they have no behaviour at all until the reference is read.

/** Byte patterns whose spelling as a number is the point. */
const SINGLES = [
    "00000000", "80000000", "3f800000", "bf800000", "40490fdb", "41200000",
    // Infinities and both NaN spellings, which the packer distinguishes.
    "7f800000", "ff800000", "7fc00000", "7fa00000",
    // The extremes, where the shortest round-tripping digit string is longest.
    "00000001", "007fffff", "00800000", "7f7fffff",
    // Straddling the switch to exponential in both directions.
    "4b7fffff", "4f000000", "5f000000", "1e000000", "0d000000", "2edbe6ff",
    "66000000", "7149f2ca", "0c000000",
];

const DOUBLES = [
    "0000000000000000", "3ff0000000000000", "bff0000000000000",
    "4005bf0a8b145769", "400921fb54442d18",
    "7ff0000000000000", "fff0000000000000", "7ff8000000000000",
    // Subnormal, maximum, and epsilon.
    "0000000000000001", "7fefffffffffffff", "3cb0000000000000",
    // 1e17 through 1e23: plain notation up to 1e20 and exponential from 1e21,
    // which is the threshold no other language shares.
    "4376345785d8a000", "43abc16d674ec800", "43e158e460913d00",
    "4415af1d78b58c40", "444b1ae4d6e2ef50", "4480f0cf064dd592", "44b52d02c7e14af6",
    // 1e-5 through 1e-8, straddling the other threshold, which is independent
    // of the first rather than its mirror.
    "3ee4f8b588e368f1", "3eb0c6f7a0b5ed8d", "3e7ad7f29abcaf48", "3e45798ee2308c3a",
    // Integers that print without a decimal point despite being large.
    "4341c37937e08000", "4350000000000000", "c3e0000000000000",
    "433fffffffffffff", "4340000000000000",
];

/** Text whose `parseFloat` prefix is not the whole token. */
const FLOAT_TEXT = [
    "1", "1.5", "-1.5", "0", "-0", ".5", "5.", "1e10", "1E10", "1e+10", "1e-10",
    // A prefix, not a parse: everything after the number is ignored.
    "1.2.3", "1,2", "12abc", "0x10", "1_000", "1e", "1e+", ".", "+.5", "-.5",
    // Not numbers at all.
    "abc", "", " ", "NaN", "Infinity", "-Infinity", "+Infinity", "infinity",
    // Leading whitespace is skipped; trailing whitespace ends the prefix.
    "  7  ", "\t3.5", "1 2 3", "1  2",
    // Beyond the representable range in both directions.
    "1e39", "-1e39", "1e-46", "1e400", "1e-400",
    // Values that land exactly between two singles, where the rounding rule
    // decides which one comes out.
    "16777217", "16777219", "1.0000001", "0.1", "3.4028235e38",
];

/** Lists whose entries repeat, including ones that look like array indices. */
const UNIQUE_LISTS = [
    "a\nb\na\nc",
    "a\na\na",
    "",
    "a",
    "\n",
    "a\n\nb",
    // Integer-like entries, which `Object.keys` hoists and reorders when the
    // count is displayed — and leaves alone when it is not.
    "b\n2\na\n1",
    "10\n9\n1\n10",
    "2\n1\n0",
    "01\n1\n01",
    "-1\n1\n0",
    "1.5\n1\n1.5",
    "\nb\n\na",
];

const SPLIT_CASES = [
    ["a,b,c", ",", "-"],
    ["a,b,c", ",", ""],
    ["abc", "", "-"],
    ["", ",", "-"],
    ["a,,b", ",", "|"],
    ["a-b-c", "-", ","],
    ["aXXbXXc", "XX", "\n"],
    ["a,b", ";", "-"],
    // The reference's own default join delimiter is a backslash and an `n`,
    // not a line feed, because the field holds literal text.
    ["a,b,c", ",", "\\n"],
];

export function add({addCase}) {
    for (const [index, hex] of SINGLES.entries()) {
        addCase(`float_single_${index}`, Buffer.from(hex, "hex"), [
            {op: "To Float", args: ["Big Endian", "Float (4 bytes)", "Space"]},
        ]);
    }
    for (const [index, hex] of DOUBLES.entries()) {
        addCase(`float_double_${index}`, Buffer.from(hex, "hex"), [
            {op: "To Float", args: ["Big Endian", "Double (8 bytes)", "Space"]},
        ]);
    }
    // Byte order and delimiter, varied against a fixed payload so a swap shows
    // up on its own rather than mixed into a formatting difference.
    for (const endian of ["Big Endian", "Little Endian"]) {
        for (const delimiter of ["Space", "Comma", "Line feed", "CRLF"]) {
            addCase(
                `float_layout_${endian}_${delimiter}`,
                Buffer.from("3f80000040490fdb", "hex"),
                [{op: "To Float", args: [endian, "Float (4 bytes)", delimiter]}],
            );
        }
    }

    for (const [index, text] of FLOAT_TEXT.entries()) {
        for (const size of ["Float (4 bytes)", "Double (8 bytes)"]) {
            const tag = size.startsWith("Float") ? "single" : "double";
            addCase(`float_parse_${tag}_${index}`, text, [
                {op: "From Float", args: ["Big Endian", size, "Space"]},
            ]);
        }
    }
    // Round trip: the bytes come back as text and then as bytes again, and the
    // corpus checks both prefixes.
    addCase("float_round_trip", Buffer.from("3f80000040490fdbc0000000", "hex"), [
        {op: "To Float", args: ["Big Endian", "Float (4 bytes)", "Space"]},
        {op: "From Float", args: ["Big Endian", "Float (4 bytes)", "Space"]},
    ]);

    for (const [index, list] of UNIQUE_LISTS.entries()) {
        for (const counted of [false, true]) {
            addCase(`unique_${index}_${counted ? "counted" : "plain"}`, list, [
                {op: "Unique", args: ["Line feed", counted]},
            ]);
        }
    }
    // Every delimiter, including the one that separates characters.
    for (const delimiter of ["Line feed", "Comma", "Space", "Nothing (separate chars)"]) {
        addCase(`unique_delim_${delimiter}`, "abcabc", [{op: "Unique", args: [delimiter, false]}]);
    }

    for (const [index, [input, from, to]] of SPLIT_CASES.entries()) {
        addCase(`split_${index}`, input, [{op: "Split", args: [from, to]}]);
    }
}
