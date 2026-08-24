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
