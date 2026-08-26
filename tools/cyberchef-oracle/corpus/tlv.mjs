// Type-Length-Value, rendered as the reference's JSON dish writes it.
//
// The output is `JSON.stringify(value, null, 4)`, which brings three
// behaviours that a hand-written serialiser gets wrong by being sensible:
//
//   - An absent key is *dropped*, not written as null. `JSON.stringify` omits
//     an object property whose value is `undefined`, so a zero key size means
//     records with two fields rather than three with one empty.
//   - A byte read past the end becomes `null` *inside the value array*, where
//     `undefined` has no spelling. Exactly one can appear: the bounds test is
//     strictly greater, so the byte at the end is still read and the next test
//     then stops.
//   - A length that overran the input is `NaN`, which `JSON.stringify` also
//     writes as `null` -- so an unreadable length is reported as no length.
//
// Truncated inputs are therefore the interesting cases, not the malformed
// ones: the reference does not fail on them, it reports what it saw.

const STREAMS = [
    // One clean record: key 01, length 02, value 03 04.
    "0102030 4".replace(/ /g, ""),
    // Two records back to back.
    "0102030401020506",
    // Zero length, so the value is empty.
    "010100",
    // Length runs past the end -- the value stops one byte over.
    "01ff0304",
    // The length byte itself is missing.
    "01",
    // Only a key byte and nothing else.
    "0101",
    // Empty input.
    "",
    // A longer value.
    "020844656c6976657279",
];

export function add({addCase}) {
    let index = 0;

    // The default shape: one byte of key, one of length.
    for (const hex of STREAMS) {
        addCase(`tlv_${index++}`, Buffer.from(hex, "hex"), [
            {op: "Parse TLV", args: [1, 1, false]},
        ]);
    }

    // No key at all, which drops the property rather than emptying it.
    for (const hex of ["0203040102", "00", ""]) {
        addCase(`tlv_nokey_${index++}`, Buffer.from(hex, "hex"), [
            {op: "Parse TLV", args: [0, 1, false]},
        ]);
    }

    // Wider fields, where the length is a little-endian sum rather than one
    // byte -- and a two-byte key.
    addCase(`tlv_wide_${index++}`, Buffer.from("01020200aabb", "hex"), [
        {op: "Parse TLV", args: [2, 2, false]},
    ]);
    addCase(`tlv_wide_${index++}`, Buffer.from("aa0400000000", "hex"), [
        {op: "Parse TLV", args: [1, 4, false]},
    ]);

    // BER, where a high bit on the first length byte turns it into a count of
    // further bytes and switches the remaining ones to big-endian.
    for (const hex of ["01020304", "018102aabb", "0181", "010204050607"]) {
        addCase(`tlv_ber_${index++}`, Buffer.from(hex, "hex"), [
            {op: "Parse TLV", args: [1, 1, true]},
        ]);
    }
}