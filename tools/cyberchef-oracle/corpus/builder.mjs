// Deterministic sampling and case accumulation for the corpus generator.
//
// Determinism is the whole point: a seeded xorshift32 PRNG, no clock, no
// `Math.random`. Every family draws from the same stream in a fixed order, so
// the generated corpus is reproducible byte for byte by anyone.

/** xorshift32. Small, seeded, and identical across engines. */
function makeRng(seed) {
    let state = seed >>> 0;
    if (state === 0) state = 0x1a2b3c4d;
    return () => {
        state ^= state << 13;
        state ^= state >>> 17;
        state ^= state << 5;
        state >>>= 0;
        return state / 0x1_0000_0000;
    };
}

/**
 * The interesting length classes for block and group codecs: empty,
 * sub-block, exact block, block+1, and a couple of larger sizes.
 */
export const LENGTHS = [0, 1, 2, 3, 4, 5, 7, 8, 11, 16, 20, 31];

function encodeInput(input) {
    return Buffer.isBuffer(input)
        ? {kind: "bytes", hex: input.toString("hex")}
        : {kind: "text", value: input};
}

/**
 * Builds the sampler and case accumulator the family modules share.
 *
 * The families receive this object and append to one list in a fixed order,
 * which keeps both the PRNG stream and the resulting case order stable.
 */
export function createBuilder({bakeString, seed}) {
    const rng = makeRng(seed);
    const cases = [];

    const randomBytes = length =>
        Buffer.from(Array.from({length}, () => Math.floor(rng() * 256)));

    const randomAscii = length => {
        const printable = [];
        for (let i = 0; i < length; i++) {
            printable.push(0x20 + Math.floor(rng() * 0x5f));
        }
        return Buffer.from(printable);
    };

    const addCase = (name, input, recipe) => {
        cases.push({name, input: encodeInput(input), recipe});
    };

    // An encode/decode pair: sample raw bytes, bake the encoder to a canonical
    // string, pin the decoder on that string. Both directions become cases.
    const encodeDecodePair = async (prefix, encoder, decoder, lengths = LENGTHS) => {
        for (const length of lengths) {
            const raw = randomBytes(length);
            addCase(`${prefix}_encode_${length}`, raw, [encoder]);
            const encoded = await bakeString(raw, [encoder]);
            addCase(`${prefix}_decode_${length}`, encoded, [decoder]);
        }
    };

    return {addCase, bakeString, cases, encodeDecodePair, randomAscii, randomBytes};
}
