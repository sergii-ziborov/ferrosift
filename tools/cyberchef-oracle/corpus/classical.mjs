// Classical ciphers.
//
// These index JavaScript strings, so the samples carry astral characters as
// well as ASCII: Vigenère counts skipped UTF-16 units to keep its key in step,
// and an emoji advances it by two rather than one. The Kelvin sign is here for
// the same reason — it is one of only two non-ASCII code points whose
// lower-case form lands in a-z, so it is treated as a letter.

const PLAIN = [
    "",
    "Hello, World!",
    "attack at dawn",
    "MiXeD CaSe 123",
    "punctuation: a-b,c.d!",
    "astral 🙂 tail",
    "Kelvin K sign",
];

const KEYS = ["a", "key", "LEMON", "zz"];

export function add({addCase}) {
    PLAIN.forEach((value, index) => {
        addCase(`atbash_${index}`, value, [{op: "Atbash Cipher", args: []}]);
        addCase(`nato_${index}`, value, [{op: "Convert to NATO alphabet", args: []}]);
        addCase(`leet_to_${index}`, value, [
            {op: "Convert Leet Speak", args: ["To Leet Speak"]},
        ]);
        addCase(`leet_from_${index}`, value, [
            {op: "Convert Leet Speak", args: ["From Leet Speak"]},
        ]);
        addCase(`leet_round_trip_${index}`, value, [
            {op: "Convert Leet Speak", args: ["To Leet Speak"]},
            {op: "Convert Leet Speak", args: ["From Leet Speak"]},
        ]);
        addCase(`rot8000_${index}`, value, [{op: "ROT8000", args: []}]);
        addCase(`rot8000_round_trip_${index}`, value, [
            {op: "ROT8000", args: []},
            {op: "ROT8000", args: []},
        ]);
        addCase(`cetacean_encode_${index}`, value, [
            {op: "Cetacean Cipher Encode", args: []},
        ]);
        addCase(`cetacean_round_trip_${index}`, value, [
            {op: "Cetacean Cipher Encode", args: []},
            {op: "Cetacean Cipher Decode", args: []},
        ]);

        for (const [a, b] of [[1, 0], [5, 8], [25, 25], [7, 3], [11, 0]]) {
            addCase(`affine_encode_${index}_${a}_${b}`, value, [
                {op: "Affine Cipher Encode", args: [a, b]},
            ]);
            addCase(`affine_round_trip_${index}_${a}_${b}`, value, [
                {op: "Affine Cipher Encode", args: [a, b]},
                {op: "Affine Cipher Decode", args: [a, b]},
            ]);
        }

        for (const key of KEYS) {
            addCase(`vigenere_encode_${index}_${key}`, value, [
                {op: "Vigenère Encode", args: [key]},
            ]);
            addCase(`vigenere_round_trip_${index}_${key}`, value, [
                {op: "Vigenère Encode", args: [key]},
                {op: "Vigenère Decode", args: [key]},
            ]);
        }

        for (const delimiter of ["Space", "Comma", "Line feed"]) {
            addCase(`a1z26_encode_${index}_${delimiter}`, value, [
                {op: "A1Z26 Cipher Encode", args: [delimiter]},
            ]);
            addCase(`a1z26_round_trip_${index}_${delimiter}`, value, [
                {op: "A1Z26 Cipher Encode", args: [delimiter]},
                {op: "A1Z26 Cipher Decode", args: [delimiter]},
            ]);
        }
    });

    // A1Z26 decode's range guard is a string-to-number comparison, so a field
    // that is not a number at all passes it and becomes a NUL byte.
    addCase("a1z26_decode_plain", "8 5 12 12 15", [{op: "A1Z26 Cipher Decode", args: ["Space"]}]);
    addCase("a1z26_decode_non_numeric", "8 xy 12", [
        {op: "A1Z26 Cipher Decode", args: ["Space"]},
    ]);
    addCase("a1z26_decode_boundaries", "1 26", [{op: "A1Z26 Cipher Decode", args: ["Space"]}]);

    // Rail fence needs a key strictly below the text length, so it is sampled
    // against a fixed message rather than the shared list.
    const RAILS = "WEAREDISCOVEREDFLEEATONCE";
    for (const key of [2, 3, 4, 7]) {
        for (const offset of [0, 1, 3]) {
            addCase(`rail_encode_${key}_${offset}`, RAILS, [
                {op: "Rail Fence Cipher Encode", args: [key, offset]},
            ]);
            addCase(`rail_round_trip_${key}_${offset}`, RAILS, [
                {op: "Rail Fence Cipher Encode", args: [key, offset]},
                {op: "Rail Fence Cipher Decode", args: [key, offset]},
            ]);
        }
    }

    // Caesar Box strips spaces before laying out the box.
    for (const height of [1, 2, 3, 5, 8]) {
        addCase(`caesar_box_${height}`, "MEET ME AT THE OLD BRIDGE", [
            {op: "Caesar Box Cipher", args: [height]},
        ]);
        addCase(`caesar_box_nospace_${height}`, "MEETMEATTHEOLDBRIDGE", [
            {op: "Caesar Box Cipher", args: [height]},
        ]);
    }

    // ROT47 runs on bytes, and zero is a documented no-op rather than a
    // rotation by the full alphabet.
    for (const amount of [0, 1, 47, 94, 95, -5]) {
        addCase(`rot47_${amount}`, Buffer.from("Hello, World! ~ \x00\x7f", "latin1"), [
            {op: "ROT47", args: [amount]},
        ]);
    }
    addCase("rot47_round_trip", Buffer.from("The Quick Brown Fox!", "latin1"), [
        {op: "ROT47", args: [47]},
        {op: "ROT47", args: [47]},
    ]);
}
