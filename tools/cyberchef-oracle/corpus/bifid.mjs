// The Bifid cipher, and the one random number generator that can be pinned.
//
// The Bifid cases are chosen around the reference's use of `String.replace`
// with a string pattern, which replaces the first match only. `JUJU jumps`
// exercises it directly: the first J folds to I and enters the square, while
// the second and third stay J, which the square has no room for, so they fall
// through to the untouched-character branch. A port that replaced every J
// would agree on one-J inputs and diverge here.
//
// Mixed case is sampled everywhere because the cipher records the case of each
// letter separately from its coordinates and restores it at the end.

const KEYWORDS = ["", "SECRET", "AAABBB"];
const TEXTS = [
    "Hello World",
    "attack at dawn",
    "MiXeD CaSe 123!",
    "JUJU jumps",
    "",
];

export function add({addCase}) {
    for (const [keyIndex, keyword] of KEYWORDS.entries()) {
        for (const [textIndex, text] of TEXTS.entries()) {
            addCase(`bifid_encode_${keyIndex}_${textIndex}`, text, [
                {op: "Bifid Cipher Encode", args: [keyword]},
            ]);
            addCase(`bifid_decode_${keyIndex}_${textIndex}`, text, [
                {op: "Bifid Cipher Decode", args: [keyword]},
            ]);
        }
    }
    // A round trip has to land back where it started, and the corpus records
    // every prefix, so this pins the intermediate too.
    addCase("bifid_round_trip", "Attack at dawn", [
        {op: "Bifid Cipher Encode", args: ["SECRET"]},
        {op: "Bifid Cipher Decode", args: ["SECRET"]},
    ]);

    addCase("xkcd_random", "anything at all", [{op: "XKCD Random Number", args: []}]);
}
