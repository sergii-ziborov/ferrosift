// Binary-coded decimal, in seven encodings and three output formats.
//
// The arithmetic is nothing -- one nibble per decimal digit -- and everything
// worth pinning is the packing. Four arguments interact: which nibble stands
// for which digit, whether two nibbles share a byte, whether a sign nibble is
// appended, and how the result is written. The corpus sweeps the combinations
// rather than sampling them, because the interactions are where the rules
// live: the sign nibble only forces a leading zero when the digits are packed
// *and* there is an even number of them, and an unpacked reading throws away
// every other nibble by a rule that reads like a mistake and is not.

const SCHEMES = [
    "8 4 2 1",
    "7 4 2 1",
    "4 2 2 1",
    "2 4 2 1",
    "8 4 -2 -1",
    "Excess-3",
    "IBM 8 4 2 1",
];

const FORMATS = ["Nibbles", "Bytes", "Raw"];

// Numbers to encode. Odd and even digit counts both matter, because the sign
// nibble is what makes them behave differently.
const NUMBERS = [
    "0", "1", "9", "12", "123", "1234", "12345",
    "-1", "-12", "-123", "-1234",
    "+5", "90210", "1000000",
    // Past 2^53, where the digits are what a float would have lost.
    "9007199254740993", "123456789012345678901234567890",
];

// An infinity passes both of the operation's guards and is encoded character
// by character, so its eight letters become eight nibbles that stand for no
// digit at all. Packed, those pack as zeros and the answer is bytes; the two
// binary renderings call a method on the missing digit and throw, so only the
// combinations that produce something are here. A fraction, a not-a-number
// and unreadable text are refused outright -- tests/conformance_bcd.rs holds
// all of those.
const INFINITE = [
    ["Infinity", true, "Bytes"],
    ["Infinity", true, "Raw"],
    ["Infinity", false, "Raw"],
    ["-Infinity", true, "Bytes"],
    ["-Infinity", true, "Raw"],
    ["-Infinity", false, "Raw"],
];

// Encoded values to read back, paired with the format they are written in.
const ENCODED = [
    ["0001 0010 0011", "Nibbles"],
    ["0000 0001 0010 0011", "Nibbles"],
    ["00010010 00110100", "Bytes"],
    ["0001 0010 0011 1101", "Nibbles"],
    ["0001 0010 0011 1100", "Nibbles"],
    // Not a multiple of four characters, so the last group is short.
    ["0001 0010 001", "Nibbles"],
    ["0001001", "Nibbles"],
    // A nibble no scheme has a digit for, and nothing at all, are both
    // refused -- tests/conformance_bcd.rs holds them.
];

export async function add({addCase}) {
    let index = 0;
    for (const number of NUMBERS) {
        for (const packed of [true, false]) {
            for (const signed of [true, false]) {
                for (const format of FORMATS) {
                    addCase(`bcd_out_${index}`, number, [
                        {op: "To BCD", args: ["8 4 2 1", packed, signed, format]},
                    ]);
                    index += 1;
                }
            }
        }
    }

    for (const [at, [number, packed, format]] of INFINITE.entries()) {
        for (const signed of [true, false]) {
            addCase(`bcd_infinite_${at}_${signed}`, number, [
                {op: "To BCD", args: ["8 4 2 1", packed, signed, format]},
            ]);
        }
    }

    // Every scheme, on one value per digit-count parity, so the tables are
    // pinned end to end rather than only where a sample landed.
    index = 0;
    for (const scheme of SCHEMES) {
        for (const number of ["1234567890", "123", "0", "-45"]) {
            for (const format of FORMATS) {
                addCase(`bcd_scheme_${index}`, number, [
                    {op: "To BCD", args: [scheme, true, false, format]},
                ]);
                index += 1;
            }
        }
    }

    index = 0;
    for (const [text, format] of ENCODED) {
        for (const packed of [true, false]) {
            for (const signed of [true, false]) {
                addCase(`bcd_in_${index}`, text, [
                    {op: "From BCD", args: ["8 4 2 1", packed, signed, format]},
                ]);
                index += 1;
            }
        }
    }
    // Numbered rather than named after the scheme: two of the seven names
    // differ only in punctuation, and stripping it collided them onto one
    // case. A duplicate name silently replaces a case rather than adding one.
    for (const [at, scheme] of SCHEMES.entries()) {
        addCase(`bcd_in_scheme_${at}`, "0001 0010 0011", [
            {op: "From BCD", args: [scheme, true, false, "Nibbles"]},
        ]);
    }

    // There and back, which is the pair's own claim about itself.
    for (const [at, format] of FORMATS.entries()) {
        for (const signed of [true, false]) {
            addCase(`bcd_round_trip_${at}_${signed}`, "12345", [
                {op: "To BCD", args: ["8 4 2 1", true, signed, format]},
                {op: "From BCD", args: ["8 4 2 1", true, signed, format]},
            ]);
            addCase(`bcd_round_trip_even_${at}_${signed}`, "1234", [
                {op: "To BCD", args: ["8 4 2 1", true, signed, format]},
                {op: "From BCD", args: ["8 4 2 1", true, signed, format]},
            ]);
        }
    }
}
