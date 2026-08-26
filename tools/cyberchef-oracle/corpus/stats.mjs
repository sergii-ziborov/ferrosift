// Three operations that report a number or read a fixed table, none of which
// pulls a dependency.
//
// Chi Square and Index of Coincidence return numbers, so what is pinned is the
// number *printed the way JavaScript prints it*: shortest round-trip decimal,
// exponential past 1e21 and below 1e-6, with a sign on the exponent that is
// never omitted. A port that formatted correctly-computed values differently
// would fail here, which is the point of sampling ratios that do not land on
// round decimals.
//
// Both clamp rather than fail on inputs too small to divide by, and both
// clamps are sampled: empty input, one letter, and two.

const TEXTS = [
    "The quick brown fox jumps over the lazy dog",
    "aaaaaaaaaaaaaaaaaaaa",
    "abcdefghijklmnopqrstuvwxyz",
    "",
    "a",
    "ab",
    "Attack at dawn!",
    "1234567890",
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit",
    "ZZZZ zzzz",
    "éèê",
];

// The sample from the operation's own description, plus the shapes around it:
// no markers at all, markers with nothing between them, and two blocks in one
// input -- where the reference's greedy match spans from the first opening
// marker to the last closing one and decodes everything between as payload.
const ENCODED = [
    "#@~^RQAAAA==-mD~sX|:/TP{~J:+dYbxL~@!F@*@!+@*@!&@*eEI@#@&@#@&.jm.raY 214Wv:zms/obI0xEAAA==^#~@",
    "no markers here at all",
    "",
    "#@~^AAAAAA==AAAAAA==^#~@",
    "prefix #@~^RQAAAA==-mD~sX|:/TP{~J:+dYbxL~@!F@*@!+@*@!&@*eEI@#@&@#@&.jm.raY 214Wv:zms/obI0xEAAA==^#~@ suffix",
];

export function add({addCase, randomBytes}) {
    let index = 0;

    for (const text of TEXTS) {
        addCase(`ioc_${index++}`, text, [{op: "Index of Coincidence", args: []}]);
        addCase(`chisq_${index++}`, text, [{op: "Chi Square", args: []}]);
    }

    // Random bytes exercise the byte histogram over the whole range, where
    // text only ever reaches the printable part of it.
    for (const size of [16, 256, 1024]) {
        addCase(`chisq_bytes_${index++}`, randomBytes(size), [
            {op: "Chi Square", args: []},
        ]);
    }

    for (const encoded of ENCODED) {
        addCase(`msscript_${index++}`, encoded, [
            {op: "Microsoft Script Decoder", args: []},
        ]);
    }
}
