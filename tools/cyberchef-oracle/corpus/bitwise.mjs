// Bit-level logic, arithmetic, shifts, rotations, and endianness.
//
// Shift counts deliberately run past the byte width: JavaScript masks them to
// five bits after a 32-bit conversion, so 8 shifts but 32 does nothing, and an
// implementation that clamps instead of masking diverges only here.

const KEYS = [
    {option: "Hex", string: "3f"},
    {option: "Hex", string: "00ff"},
    {option: "UTF8", string: "key"},
    {option: "Hex", string: ""},
];

const KEYED = ["AND", "OR", "ADD", "SUB"];

export function add({addCase, randomBytes}) {
    for (const length of [0, 1, 5, 16, 64]) {
        const raw = randomBytes(length);
        addCase(`not_${length}`, raw, [{op: "NOT", args: []}]);
        KEYED.forEach((op, keyIndex) => {
            for (const key of KEYS) {
                const name = `${op.toLowerCase()}_${length}_${key.option}${key.string.length}`;
                addCase(name, raw, [{op, args: [key]}]);
            }
            // Compose against the neighbouring operator, to pin the pair.
            addCase(`${op.toLowerCase()}_compose_${length}`, raw, [
                {op, args: [KEYS[keyIndex % KEYS.length]]},
                {op: KEYED[(keyIndex + 1) % KEYED.length], args: [KEYS[0]]},
            ]);
        });

        // Bit shift left constrains its amount to 0..=7 and the reference
        // rejects anything outside that before running, so only valid amounts
        // are sampled here; the rejection is pinned in conformance_bitwise.rs.
        for (const amount of [0, 1, 3, 7]) {
            addCase(`shift_left_${length}_${amount}`, raw, [
                {op: "Bit shift left", args: [amount]},
            ]);
        }
        for (const amount of [0, 1, 3, 7, 8, 31, 32]) {
            addCase(`shift_right_logical_${length}_${amount}`, raw, [
                {op: "Bit shift right", args: [amount, "Logical shift"]},
            ]);
            addCase(`shift_right_arith_${length}_${amount}`, raw, [
                {op: "Bit shift right", args: [amount, "Arithmetic shift"]},
            ]);
        }

        for (const amount of [0, 1, 3, 7, 8, 9]) {
            for (const carry of [false, true]) {
                const suffix = carry ? "carry" : "plain";
                addCase(`rotate_left_${suffix}_${length}_${amount}`, raw, [
                    {op: "Rotate left", args: [amount, carry]},
                ]);
                addCase(`rotate_right_${suffix}_${length}_${amount}`, raw, [
                    {op: "Rotate right", args: [amount, carry]},
                ]);
            }
        }

        addCase(`ror13_${length}`, raw, [{op: "ROR13", args: []}]);
    }

    // Round trips that must land back on the input.
    const sample = randomBytes(24);
    addCase("not_round_trip", sample, [{op: "NOT", args: []}, {op: "NOT", args: []}]);
    addCase("add_sub_round_trip", sample, [
        {op: "ADD", args: [{option: "Hex", string: "7f"}]},
        {op: "SUB", args: [{option: "Hex", string: "7f"}]},
    ]);
    addCase("rotate_round_trip", sample, [
        {op: "Rotate left", args: [3, false]},
        {op: "Rotate right", args: [3, false]},
    ]);
    addCase("rotate_carry_round_trip", sample, [
        {op: "Rotate left", args: [3, true]},
        {op: "Rotate right", args: [3, true]},
    ]);

    // Swap endianness reads and writes text, and its Hex reader is the
    // permissive library one: `0x` prefixes, mixed separators, an odd trailing
    // digit, and an empty input all have defined answers.
    const HEX_INPUTS = [
        "",
        "0a1b2c3d",
        "0x0a 0x1b 0x2c 0x3d",
        "0a1b2c3d4e",
        "0a,1b;2c:3d",
        "abc",
    ];
    HEX_INPUTS.forEach((value, index) => {
        for (const [width, pad] of [[4, true], [4, false], [2, true], [1, true], [8, false]]) {
            addCase(`swap_hex_${index}_${width}_${pad}`, value, [
                {op: "Swap endianness", args: ["Hex", width, pad]},
            ]);
        }
    });
    for (const [name, value] of [
        ["ascii", "abcdefgh"],
        ["short", "abc"],
        ["empty", ""],
        ["latin", "éèêë"],
    ]) {
        addCase(`swap_raw_${name}`, value, [{op: "Swap endianness", args: ["Raw", 4, true]}]);
        addCase(`swap_raw_nopad_${name}`, value, [
            {op: "Swap endianness", args: ["Raw", 3, false]},
        ]);
    }
}
