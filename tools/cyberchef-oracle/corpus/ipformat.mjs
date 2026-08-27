// One IPv4 address, written four different ways.
//
// The conversion itself is four bytes and some shifts. What the corpus is for
// is the edges around it, and there are several: the operation drops empty
// lines rather than preserving them, copies a line through untouched when the
// two formats are the same, and never checks that a dotted address has four
// pieces.
//
// The thirty-two bit wrap is the one worth pinning hardest. The reference
// builds its octets with JavaScript shifts, which truncate, so a decimal above
// four billion is not refused -- it wraps. A port that used a wider integer
// would answer differently and look more correct doing it.

const FORMATS = ["Dotted Decimal", "Decimal", "Octal", "Hex"];

const ADDRESSES = {
    "Dotted Decimal": [
        "192.168.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "8.8.8.8",
        "127.0.0.1",
        // Not four pieces. The reference does not check, and writes back
        // however many it was given.
        "1.2.3",
        "1.2.3.4.5",
        "10",
        // Pieces that are not numbers, or are numbers with something after
        // them: `parseInt` reads a leading number and ignores the rest.
        "1.2.3.x",
        "1.2.3.4x",
        "1.2.3.",
        ".1.2.3",
        // Octets past a byte, which the reference masks rather than refuses.
        "999.999.999.999",
        "256.1.1.1",
        // A negative piece.
        "-1.2.3.4",
    ],
    Decimal: [
        "3232235521",
        "0",
        "4294967295",
        "134744072",
        "2130706433",
        // Past thirty-two bits, where the shift wraps.
        "4294967296",
        "9999999999",
        // Not a number, and a number with something after it.
        "abc",
        "123abc",
        "",
        "-1",
    ],
    Octal: [
        "030052000001",
        "0",
        "037777777777",
        "01000000010",
        // A leading zero is what the reference writes, and `parseInt` with
        // radix eight reads it either way.
        "30052000001",
        "08",
        "0777777777777",
    ],
    Hex: [
        "c0a80001",
        "00000000",
        "ffffffff",
        "08080808",
        "7f000001",
        // Fewer than eight digits, and more.
        "c0a8",
        "c0a800010203",
        // An odd number of digits, which pairs off and drops the last.
        "c0a8000",
        // Separators, which the reference's hex reading treats as delimiters.
        "c0 a8 00 01",
        "c0:a8:00:01",
        "zz",
    ],
};

export async function add({addCase}) {
    let index = 0;
    for (const [inFormat, samples] of Object.entries(ADDRESSES)) {
        for (const sample of samples) {
            for (const outFormat of FORMATS) {
                addCase(`ipformat_${index}`, sample, [
                    {op: "Change IP format", args: [inFormat, outFormat]},
                ]);
                index += 1;
            }
        }
    }

    // Several lines at once, including empty ones, which are dropped rather
    // than preserved -- so the answer has fewer lines than the input.
    addCase("ipformat_lines", "192.168.0.1\n8.8.8.8\n\n127.0.0.1", [
        {op: "Change IP format", args: ["Dotted Decimal", "Decimal"]},
    ]);
    addCase("ipformat_lines_blank_first", "\n192.168.0.1\n\n\n8.8.8.8\n", [
        {op: "Change IP format", args: ["Dotted Decimal", "Hex"]},
    ]);
    addCase("ipformat_empty", "", [
        {op: "Change IP format", args: ["Dotted Decimal", "Decimal"]},
    ]);

    // Same format in and out: the line is copied through without being read,
    // so even a malformed one survives.
    for (const [at, format] of FORMATS.entries()) {
        addCase(`ipformat_same_${at}`, "not an address at all", [
            {op: "Change IP format", args: [format, format]},
        ]);
    }

    // There and back, in every pairing.
    let round = 0;
    for (const from of FORMATS) {
        for (const to of FORMATS) {
            addCase(`ipformat_round_${round}`, "192.168.0.1", [
                {op: "Change IP format", args: ["Dotted Decimal", from]},
                {op: "Change IP format", args: [from, to]},
                {op: "Change IP format", args: [to, "Dotted Decimal"]},
            ]);
            round += 1;
        }
    }
}
