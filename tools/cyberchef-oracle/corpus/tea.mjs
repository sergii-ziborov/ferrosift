// TEA and XTEA, in five modes, five padding schemes and both directions.
//
// The block functions are published and have test vectors, so they are the
// part least likely to be wrong. What the corpus is actually for is everything
// wrapped around them, all of which is the reference's own arrangement:
//
//   * The three stream modes read a whole block at the end, zero-filling what
//     the message does not reach, and then cut the output back to the message
//     length. A port that stopped at the message instead produces the same
//     bytes for aligned input and different bytes for every other length --
//     which is why every length either side of a block boundary is here.
//   * ZERO and RANDOM padding are added on the way in and *not* removed on the
//     way out, so a round trip through either returns more than it was given.
//   * An empty message is empty output before anything is padded, so NO padding
//     does not refuse it.
//   * The `Raw` output is not the ciphertext bytes. It goes through
//     `byteArrayToUtf8`, which decodes them as UTF-8 and falls back to one
//     character per byte -- so what a recipe reads next is text either way.
//
// Decryption is pinned on the reference's own ciphertext, baked at generation
// time, so a decrypt case can never drift from its encrypt case.

const KEY = {option: "Hex", string: "00112233445566778899aabbccddeeff"};
const IV = {option: "Hex", string: "0102030405060708"};
const NO_IV = {option: "Hex", string: ""};

const MODES = ["CBC", "CFB", "OFB", "CTR", "ECB"];

// Around the block boundary on both sides, plus the empty message.
const LENGTHS = [0, 1, 7, 8, 9, 16, 17];

// Standard, one, and past what a byte holds signed -- the interface allows
// anything from one to two hundred and fifty-five.
const CYCLES = [1, 8, 32, 64, 255];

/** The argument list for one of the four operations. */
const args = ({key = KEY, iv = IV, mode = "CBC", input = "Raw", output = "Hex",
    padding = "PKCS5", cycles}) => {
    const list = [key, iv, mode, input, output, padding];
    if (cycles !== undefined) list.push(cycles);
    return list;
};

export async function add({addCase, bakeString, randomAscii}) {
    let index = 0;

    // Both ciphers, every mode, every length. Printable ASCII rather than
    // random bytes: the reference reads this input as a *string* and converts
    // it back, and ASCII is where that round trip is an identity for certain.
    for (const [op, cycles] of [["TEA Encrypt", undefined], ["XTEA Encrypt", 32]]) {
        for (const mode of MODES) {
            for (const length of LENGTHS) {
                const plain = randomAscii(length);
                addCase(`tea_${index}`, plain, [{op, args: args({mode, cycles})}]);
                index += 1;
            }
        }
    }

    // The same, decrypting, pinned on what the reference just produced.
    index = 0;
    for (const [encrypt, decrypt, cycles] of [
        ["TEA Encrypt", "TEA Decrypt", undefined],
        ["XTEA Encrypt", "XTEA Decrypt", 32],
    ]) {
        for (const mode of MODES) {
            for (const length of LENGTHS) {
                const plain = randomAscii(length);
                const cipher = await bakeString(plain, [
                    {op: encrypt, args: args({mode, cycles})},
                ]);
                addCase(`tea_back_${index}`, cipher, [
                    {op: decrypt, args: args({mode, input: "Hex", output: "Raw", cycles})},
                ]);
                index += 1;
            }
        }
    }

    // Padding, in the two modes that pad. NO and RANDOM only on lengths that
    // need no padding: the first refuses otherwise and the second fills with
    // `Math.random`, so neither has an answer to pin off a block boundary.
    index = 0;
    for (const padding of ["PKCS5", "ZERO", "BIT", "NO", "RANDOM"]) {
        const lengths = padding === "NO" || padding === "RANDOM" ? [0, 8, 16] : LENGTHS;
        for (const mode of ["ECB", "CBC"]) {
            for (const length of lengths) {
                const plain = randomAscii(length);
                addCase(`tea_pad_${index}`, plain, [
                    {op: "TEA Encrypt", args: args({mode, padding})},
                ]);
                index += 1;

                // And back again, which is where ZERO and RANDOM show that they
                // return more than they were given.
                //
                // Except BIT on a message that is already a whole number of
                // blocks, which does not round-trip in the reference either.
                // `applyPadding` returns early for every scheme but PKCS5 when
                // nothing needs adding, so no marker is written -- and the
                // removal then scans back for a `0x80` that was never there and
                // throws. There is no output to pin; `conformance_tea.rs`
                // asserts that this refuses too.
                const roundTrips = !(padding === "BIT" && length % 8 === 0 && length > 0);
                if (roundTrips) {
                    const cipher = await bakeString(plain, [
                        {op: "TEA Encrypt", args: args({mode, padding})},
                    ]);
                    addCase(`tea_pad_back_${index}`, cipher, [
                        {
                            op: "TEA Decrypt",
                            args: args({mode, input: "Hex", output: "Raw", padding}),
                        },
                    ]);
                }
                index += 1;
            }
        }
    }

    // XTEA's cycle count, which changes the cipher rather than a detail of it.
    index = 0;
    for (const cycles of CYCLES) {
        for (const mode of ["ECB", "CTR"]) {
            const plain = randomAscii(24);
            addCase(`tea_cycles_${index}`, plain, [
                {op: "XTEA Encrypt", args: args({mode, cycles})},
            ]);
            const cipher = await bakeString(plain, [
                {op: "XTEA Encrypt", args: args({mode, cycles})},
            ]);
            addCase(`tea_cycles_back_${index}`, cipher, [
                {op: "XTEA Decrypt", args: args({mode, input: "Hex", output: "Raw", cycles})},
            ]);
            index += 1;
        }
    }

    // The input and output encodings, and an absent IV -- which is eight null
    // bytes rather than an error, and is ignored outright in ECB.
    index = 0;
    for (const input of ["Raw", "Hex"]) {
        for (const output of ["Hex", "Raw"]) {
            const plain = input === "Hex" ? "48656c6c6f2c20776f726c6421" : randomAscii(13);
            addCase(`tea_format_${index}`, plain, [
                {op: "TEA Encrypt", args: args({input, output})},
            ]);
            index += 1;
        }
    }
    for (const mode of MODES) {
        addCase(`tea_noiv_${index}`, randomAscii(16), [
            {op: "TEA Encrypt", args: args({iv: NO_IV, mode})},
        ]);
        index += 1;
    }

    // A hex key with a separator in it, so the field's reading is exercised
    // here too rather than only in `togglestring.mjs`.
    addCase("tea_spaced_key", randomAscii(16), [
        {
            op: "TEA Encrypt",
            args: args({key: {option: "Hex", string: "00 11 22 33 44 55 66 77 88 99 aa bb cc dd ee ff"}}),
        },
    ]);
}
