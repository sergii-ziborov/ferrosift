// Windows filetimes and UNIX timestamps.
//
// The arithmetic is a multiplication and one addition, so what is actually
// worth pinning is everything around it: the units, the three output formats,
// and the endianness swap -- which is transcribed rather than tidied, because
// the two directions are *not* inverses of each other for an odd-length
// string. One appends `0` and the first character; the other moves the last
// character to the front. A cleaned-up implementation would round-trip and
// disagree with the reference.

const UNITS = [
    "Seconds (s)",
    "Milliseconds (ms)",
    // U+03BC, the Greek letter, not U+00B5, the micro sign. They look
    // identical and the reference uses the first.
    "Microseconds (μs)",
    "Nanoseconds (ns)",
];

const FORMATS = ["Decimal", "Hex (big endian)", "Hex (little endian)"];

// Timestamps, in whatever unit the case pairs them with.
const TIMESTAMPS = [
    "0", "1", "1700000000", "1234567890",
    // Before the UNIX epoch, and before the filetime epoch, where the value
    // goes negative and the hexadecimal rendering carries a sign.
    "-1", "-11644473600", "-99999999999999999999",
    // Fractional, which the multiplication keeps and the division rounds.
    "1.5", "0.0000001", "1700000000.123456789",
    // Past 2^53 in the input as well as in the answer.
    "9007199254740993", "1e21",
    // The specials, which the constructor reads and the arithmetic carries.
    "NaN", "Infinity", "-Infinity",
];

// Filetimes, written the way the matching format expects.
const FILETIMES = [
    ["116444736000000000", "Decimal"],
    ["133444736000000000", "Decimal"],
    ["0", "Decimal"],
    ["-1", "Decimal"],
    ["19db1ded53e8000", "Hex (big endian)"],
    ["1DA1747C66D0000", "Hex (big endian)"],
    ["19DB1DED53E8000", "Hex (big endian)"],
    // A mixed-case value and text that is no number at all both make the
    // reference *throw*, so they cannot be pinned as output here --
    // tests/conformance_filetime.rs holds them.
    // An odd number of characters, which is where the two swaps part company.
    ["19db1ded53e8000", "Hex (little endian)"],
    ["0080e83ced1d9b01", "Hex (little endian)"],
    ["abcde", "Hex (little endian)"],
    ["abcd", "Hex (little endian)"],
    ["a", "Hex (little endian)"],
    ["", "Decimal"],
    ["", "Hex (little endian)"],
];

export async function add({addCase}) {
    let index = 0;
    for (const timestamp of TIMESTAMPS) {
        for (const units of UNITS) {
            for (const format of FORMATS) {
                addCase(`filetime_out_${index}`, timestamp, [
                    {op: "UNIX Timestamp to Windows Filetime", args: [units, format]},
                ]);
                index += 1;
            }
        }
    }

    index = 0;
    for (const [filetime, format] of FILETIMES) {
        for (const units of UNITS) {
            addCase(`filetime_in_${index}`, filetime, [
                {op: "Windows Filetime to UNIX Timestamp", args: [units, format]},
            ]);
            index += 1;
        }
    }

    // There and back, in each format. The odd-length swap means this is a real
    // question rather than a tautology: a value whose hexadecimal rendering has
    // an odd number of characters does not survive the round trip, and the
    // corpus records whatever the reference actually produces.
    for (const [at, format] of FORMATS.entries()) {
        addCase(`filetime_round_trip_${at}`, "1700000000", [
            {op: "UNIX Timestamp to Windows Filetime", args: ["Seconds (s)", format]},
            {op: "Windows Filetime to UNIX Timestamp", args: ["Seconds (s)", format]},
        ]);
        addCase(`filetime_round_trip_odd_${at}`, "1", [
            {op: "UNIX Timestamp to Windows Filetime", args: ["Nanoseconds (ns)", format]},
            {op: "Windows Filetime to UNIX Timestamp", args: ["Nanoseconds (ns)", format]},
        ]);
    }
}
