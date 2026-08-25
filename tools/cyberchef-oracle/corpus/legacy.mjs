// SHA-0 and MurmurHash3.
//
// The SHA-0 lengths are chosen for the padding rule, not for variety: 55 is
// the last length that fits its length field in the same block, 56 is the
// first that does not, and 64 is a whole block with nothing left over. A
// padding implementation that is wrong is wrong at exactly those three.
//
// MurmurHash3 is sampled at every remainder class mod 4, because the tail is
// handled by a separate fall-through that mixes one, two, or three bytes and
// is skipped entirely at zero. Seeds and the signed flag vary independently:
// the flag only reinterprets the finished hash, so it needs a case whose top
// bit is set to show anything at all.

const MURMUR_TEXT = [
    "",
    "a",
    "ab",
    "abc",
    "abcd",
    "abcde",
    "Hello, World!",
    "The quick brown fox jumps over the lazy dog",
];

export function add({addCase, randomBytes}) {
    for (const length of [0, 1, 55, 56, 63, 64, 65, 119, 120]) {
        addCase(`sha0_${length}`, randomBytes(length), [{op: "SHA0", args: [80]}]);
    }

    for (const [index, text] of MURMUR_TEXT.entries()) {
        for (const seed of [0, 1, 0x9747_b28c]) {
            for (const signed of [false, true]) {
                addCase(`murmur3_${index}_${seed}_${signed ? "s" : "u"}`, text, [
                    {op: "MurmurHash3", args: [seed, signed]},
                ]);
            }
        }
    }
}
