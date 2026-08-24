// Checksums, Modhex, and Morse.
//
// The lengths are chosen for the word boundaries these algorithms fold on:
// Fletcher-32 reads 16-bit words and Fletcher-64 reads 32-bit ones, and both
// assemble a trailing partial word by a rule of their own, so odd lengths and
// lengths one short of a word are the interesting ones.

const LENGTHS = [0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 64];

export function add({addCase, randomBytes}) {
    for (const length of LENGTHS) {
        const raw = randomBytes(length);
        addCase(`adler32_${length}`, raw, [{op: "Adler-32 Checksum", args: []}]);
        for (const width of [8, 16, 32, 64]) {
            addCase(`fletcher${width}_${length}`, raw, [
                {op: `Fletcher-${width} Checksum`, args: []},
            ]);
        }
        addCase(`tcpip_${length}`, raw, [{op: "TCP/IP Checksum", args: []}]);
        for (const block of [1, 2, 4, 8, 16]) {
            addCase(`xor_checksum_${length}_${block}`, raw, [
                {op: "XOR Checksum", args: [block]},
            ]);
        }

        for (const [delimiter, perLine] of [
            ["Space", 0],
            ["None", 0],
            ["Comma", 0],
            ["Space", 4],
            ["Colon", 3],
        ]) {
            addCase(`modhex_encode_${length}_${delimiter}_${perLine}`, raw, [
                {op: "To Modhex", args: [delimiter, perLine]},
            ]);
            addCase(`modhex_round_trip_${length}_${delimiter}_${perLine}`, raw, [
                {op: "To Modhex", args: [delimiter, perLine]},
                {op: "From Modhex", args: ["Auto"]},
            ]);
        }
    }

    // Modhex decoding with an explicit delimiter, where a stray character is
    // not treated as a separator.
    for (const [name, value] of [
        ["clean", "cbdefghi"],
        ["spaced", "cb de fg hi"],
        ["mixed_case", "CBdeFGhi"],
        ["stray", "cb!de"],
        ["empty", ""],
    ]) {
        addCase(`modhex_decode_auto_${name}`, value, [{op: "From Modhex", args: ["Auto"]}]);
        addCase(`modhex_decode_none_${name}`, value, [{op: "From Modhex", args: ["None"]}]);
    }

    // Luhn reports three lines, and its radix must be even. Every character
    // has to be a valid digit in that radix, so binary is sampled only against
    // input that is actually binary.
    for (const [name, value] of [
        ["digits", "4485275742308327"],
        ["short", "7"],
        ["zeros", "0000"],
        ["empty", ""],
    ]) {
        for (const radix of [10, 16, 36]) {
            addCase(`luhn_${name}_${radix}`, value, [{op: "Luhn Checksum", args: [radix]}]);
        }
    }
    addCase("luhn_binary_2", "10110", [{op: "Luhn Checksum", args: [2]}]);
    addCase("luhn_hex_letters_16", "deadBEEF", [{op: "Luhn Checksum", args: [16]}]);

    // Morse: every format option, several delimiter pairs, and text that
    // exercises unknown characters, runs of spaces, and multiple lines.
    const MORSE_TEXT = [
        "",
        "SOS",
        "hello world",
        "MIXED case 123",
        "spaced    out",
        "line one\nline two",
        "punctuation .,:?!",
        "unknown ~ chars",
    ];
    const FORMATS = ["-/.", "_/.", "Dash/Dot", "DASH/DOT", "dash/dot"];
    MORSE_TEXT.forEach((value, index) => {
        FORMATS.forEach(format => {
            addCase(`morse_encode_${index}_${format.replace(/\W/g, "")}`, value, [
                {op: "To Morse Code", args: [format, "Space", "Line feed"]},
            ]);
        });
        // A space is offered between letters but not between words, so
        // `Forward slash` is the second word delimiter sampled here.
        for (const [letter, word] of [
            ["Space", "Line feed"],
            ["Space", "Forward slash"],
            ["Comma", "Line feed"],
        ]) {
            addCase(`morse_round_trip_${index}_${letter}_${word}`.replace(/\s/g, ""), value, [
                {op: "To Morse Code", args: ["-/.", letter, word]},
                {op: "From Morse Code", args: [letter, word]},
            ]);
        }
    });

    // Decoding accepts several dash and dot spellings, and drops signals it
    // does not recognise rather than failing.
    for (const [name, value] of [
        ["hyphen", "... --- ..."],
        ["underscore", "___ ... ___"],
        ["words", "dot dot dot dash dash dash"],
        ["middot", "·· −−"],
        ["unknown", "........... ..."],
    ]) {
        addCase(`morse_decode_${name}`, value, [
            {op: "From Morse Code", args: ["Space", "Line feed"]},
        ]);
    }
}
