// Byte-level editing: XOR, hexdump, slicing, search and replace.

export const byteCases = [
    {
        name: "xor_hex_standard",
        input: {kind: "bytes", hex: "48656c6c6f"},
        recipe: [{op: "XOR", args: [{option: "Hex", string: "0f"}, "Standard", false]}],
    },
    {
        name: "xor_null_preserving",
        input: {kind: "bytes", hex: "00010002"},
        recipe: [{op: "XOR", args: [{option: "Hex", string: "01"}, "Standard", true]}],
    },
    {
        name: "xor_cascade",
        input: {kind: "bytes", hex: "01020304"},
        recipe: [{op: "XOR", args: [{option: "Hex", string: "ff"}, "Cascade", false]}],
    },
    // A toggleString field is read into a plain JavaScript array, and only the
    // Decimal and Binary options can put something in it that a byte cannot
    // hold. `bitOp` never coerces the key: it hands it to `^`, `&`, `|` or
    // plain arithmetic and pushes the result, and what happens next is decided
    // by `Dish.valid()`, which refuses an element `< 0` or `> 255` outright.
    //
    // So the same out-of-range key succeeds or fails depending on the operator,
    // and these are the cases where it succeeds. The failures are in
    // `conformance_togglestring.rs`: this fixture records reference *output*,
    // and a recipe the reference refuses to run has none.
    //
    // NaN is the one value that is out of range and still allowed, because the
    // check is a pair of comparisons and NaN fails both. It reaches the array,
    // survives validation, and becomes zero when the array is finally stored --
    // and only ADD and SUB can produce it, since the bitwise three convert it
    // away with ToInt32 first. That is why the same key leaves the input alone
    // under OR and erases it under ADD.
    {
        name: "xor_decimal_key_overflows_to_infinity",
        // A run of digits long enough to overflow a double is Infinity rather
        // than an error, and ToInt32 of Infinity is zero.
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [
            {op: "XOR", args: [{option: "Decimal", string: "9".repeat(400)}, "Standard", false]},
        ],
    },
    {
        name: "xor_binary_key_not_a_number",
        // Eight binary digits cannot exceed 255, so a Binary field reaches the
        // consumer out of range only as NaN -- here from a chunk that starts on
        // a character `parseInt` will not read.
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [{op: "XOR", args: [{option: "Binary", string: "2"}, "Standard", false]}],
    },
    {
        name: "add_decimal_key_not_a_number",
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [{op: "ADD", args: [{option: "Decimal", string: "-"}]}],
    },
    {
        name: "sub_decimal_key_not_a_number",
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [{op: "SUB", args: [{option: "Decimal", string: "-"}]}],
    },
    {
        name: "and_decimal_key_not_a_number",
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [{op: "AND", args: [{option: "Decimal", string: "-"}]}],
    },
    {
        name: "or_decimal_key_not_a_number",
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [{op: "OR", args: [{option: "Decimal", string: "-"}]}],
    },
    {
        name: "add_decimal_key_above_byte_range",
        // `(o + 300) % 256` lands in range for every byte, so 300 is an
        // ordinary key here and the same key breaks OR entirely.
        input: {kind: "bytes", hex: "0102ff"},
        recipe: [{op: "ADD", args: [{option: "Decimal", string: "300"}]}],
    },
    {
        name: "sub_decimal_key_above_byte_range",
        // `o - 300` is negative for every byte, and the correction adds 256
        // once -- which is enough only for a byte of 44 or more. Every byte
        // here clears that; one below it would fail the dish.
        input: {kind: "bytes", hex: "2c40ff"},
        recipe: [{op: "SUB", args: [{option: "Decimal", string: "300"}]}],
    },
    {
        name: "and_decimal_key_above_byte_range",
        // `&` can only clear bits, so an out-of-range key is always in range
        // by the time it is a result.
        input: {kind: "bytes", hex: "2c2d00ff"},
        recipe: [{op: "AND", args: [{option: "Decimal", string: "300"}]}],
    },
    {
        name: "hexdump_round_trip",
        input: {kind: "bytes", hex: "466572726f53696674"},
        recipe: [
            {op: "To Hexdump", args: [16, false, false, false]},
            {op: "From Hexdump", args: []},
        ],
    },
    {
        name: "to_hexdump_upper_final",
        input: {kind: "bytes", hex: "4142"},
        recipe: [{op: "To Hexdump", args: [8, true, true, false]}],
    },
    {
        name: "take_and_drop_bytes",
        input: {kind: "bytes", hex: "6162636465666768"},
        recipe: [
            {op: "Take bytes", args: [2, 3, false]},
            {op: "Drop bytes", args: [1, 1, false]},
        ],
    },
    {
        name: "head_line_feed",
        input: {kind: "text", value: "a\nb\nc\nd"},
        recipe: [{op: "Head", args: ["Line feed", 2]}],
    },
    {
        name: "find_replace_simple_and_extended",
        input: {kind: "text", value: "foo\tbar foo"},
        recipe: [
            {
                op: "Find / Replace",
                args: [
                    {option: "Simple string", string: "foo"},
                    "x",
                    true,
                    false,
                    true,
                    false,
                ],
            },
            {
                op: "Find / Replace",
                args: [
                    {option: "Extended (\\n, \\t, \\x...)", string: "\\t"},
                    "-",
                    true,
                    false,
                    true,
                    false,
                ],
            },
        ],
    },
];
