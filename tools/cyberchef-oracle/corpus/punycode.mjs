// Punycode, RFC 3492, in both of the shapes the reference exposes.
//
// The flag switches between two genuinely different operations. Off, the input
// is one label and the transform is bare RFC 3492 with no prefix. On, the
// input is a domain: labels split, non-ASCII ones gain `xn--`, and ASCII ones
// pass through untouched.
//
// Three quirks of the domain wrapper are pinned rather than assumed:
//
//   - `a@b@c` loses the tail. The reference splits on every `@` and reads only
//     the first two pieces, so the third is discarded silently.
//   - Four characters separate labels on the way in -- `.`, U+3002, U+FF0E and
//     U+FF61 -- and all four come out as `.`, so the separator that was typed
//     is not the one returned.
//   - `xn--` is matched case-sensitively but its payload is lower-cased before
//     decoding, so `xn--MNCHEN-3YA` decodes and `XN--mnchen-3ya` does not.

const ENCODE_LABELS = [
    "münchen",
    "bücher",
    "مثال",
    "例子",
    "рф",
    "ascii-only",
    "a",
    "",
    "münchen-mit-bindestrich",
    "😀",
];

const DECODE_LABELS = [
    "mnchen-3ya",
    "bcher-kva",
    "mgbh0fb",
    "fsqu00a",
    "p1ai",
    "ascii-only",
    "a",
    "",
];

const DOMAINS_IN = [
    "münchen.de",
    "bücher.example.com",
    "example.com",
    "user@münchen.de",
    "a@b@c",
    "münchen。de",
    "münchen．de",
    "münchen｡de",
    "рф.рф",
];

const DOMAINS_OUT = [
    "xn--mnchen-3ya.de",
    "xn--bcher-kva.example.com",
    "example.com",
    "user@xn--mnchen-3ya.de",
    "xn--MNCHEN-3YA.de",
    "XN--mnchen-3ya.de",
    "xn--p1ai.xn--p1ai",
];

export function add({addCase}) {
    let index = 0;
    for (const label of ENCODE_LABELS) {
        addCase(`punycode_enc_${index++}`, label, [
            {op: "To Punycode", args: [false]},
        ]);
    }
    for (const label of DECODE_LABELS) {
        addCase(`punycode_dec_${index++}`, label, [
            {op: "From Punycode", args: [false]},
        ]);
    }
    for (const domain of DOMAINS_IN) {
        addCase(`punycode_idn_enc_${index++}`, domain, [
            {op: "To Punycode", args: [true]},
        ]);
    }
    for (const domain of DOMAINS_OUT) {
        addCase(`punycode_idn_dec_${index++}`, domain, [
            {op: "From Punycode", args: [true]},
        ]);
    }
}
