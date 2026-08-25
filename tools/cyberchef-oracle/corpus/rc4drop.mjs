// RC4 with the leading keystream discarded.
//
// The drop count is in 32-bit words, not bytes, so 192 skips 768 keystream
// bytes. A port that read the count as bytes agrees with the reference only
// when the figure is a multiple of four -- and 192 is one, which is exactly
// why the default alone proves nothing. The counts below include 1, 3 and 5 so
// the unit is pinned rather than coincidentally matched.
//
// Zero and a negative count both mean "drop nothing": the reference counts
// down to zero, so a negative figure never enters the loop.

const DROPS = [0, 1, 2, 3, 5, 192, 768];
const KEYS = [
    {option: "UTF8", string: "secret"},
    {option: "Hex", string: "00ff10"},
    {option: "UTF8", string: ""},
];
const MESSAGES = ["Attack at dawn", "a", "", "0123456789abcdef0123456789"];

export function add({addCase}) {
    let index = 0;
    for (const drop of DROPS) {
        for (const message of MESSAGES) {
            addCase(`rc4_drop_${index++}`, message, [
                {op: "RC4 Drop", args: [KEYS[0], "Latin1", "Hex", drop]},
            ]);
        }
    }
    for (const key of KEYS) {
        addCase(`rc4_drop_key_${index++}`, "The quick brown fox", [
            {op: "RC4 Drop", args: [key, "Latin1", "Hex", 192]},
        ]);
    }
    for (const format of ["Latin1", "UTF8", "Hex", "Base64"]) {
        addCase(`rc4_drop_out_${index++}`, "0123456789abcdef", [
            {op: "RC4 Drop", args: [KEYS[0], "Hex", format, 192]},
        ]);
    }
}
