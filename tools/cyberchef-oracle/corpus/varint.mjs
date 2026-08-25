// VarInt coding and quoted-printable decoding.

// VarInt boundaries are at every multiple of seven bits, because that is where
// a byte is added. Each of these sits either side of one.
const VARINT_NUMBERS = [
    "0",
    "1",
    "127",
    "128",
    "129",
    "255",
    "300",
    "16383",
    "16384",
    "2097151",
    "2097152",
    "4294967295",
    "18446744073709551615",
    // The BigInt constructor accepts radix prefixes and surrounding space.
    "0x7f",
    "0xFF",
    "0b1010",
    "0o777",
    "  42  ",
    "+7",
    "",
];

const QUOTED_PRINTABLE = [
    "Hello=20World",
    "caf=C3=A9",
    // Soft line breaks, in both line-ending styles and at end of input.
    "long line that was=\r\nwrapped",
    "long line that was=\nwrapped",
    "trailing soft break=",
    // An equals that is not an escape: too few digits, or not hex.
    "100=",
    "=ZZ not hex",
    "=4 short",
    "plain text with no escapes",
    "",
    "=3D is an escaped equals",
    // Lower and upper case hex digits, which the reference accepts either way.
    "=c3=a9 and =C3=A9",
];

export async function add({addCase, bakeString}) {
    for (const [index, value] of VARINT_NUMBERS.entries()) {
        addCase(`varint_encode_${index}`, value, [{op: "VarInt Encode", args: []}]);
        // Round trip: encoding then decoding must give the number back in its
        // canonical decimal form, whatever radix it went in as.
        const encoded = await bakeString(value, [{op: "VarInt Encode", args: []}]);
        addCase(
            `varint_round_trip_${index}`,
            Buffer.from(encoded, "latin1"),
            [{op: "VarInt Decode", args: []}],
        );
    }

    // Decoding raw byte sequences, including a buffer holding more than one
    // VarInt — the reference stops at the first and ignores the rest.
    for (const [index, bytes] of [
        [0x00],
        [0x01],
        [0x7f],
        [0x80, 0x01],
        [0xff, 0x01],
        [0xac, 0x02],
        [0xff, 0xff, 0xff, 0xff, 0x0f],
        [0x96, 0x01, 0x96, 0x01],
        [0x80],
    ].entries()) {
        addCase(`varint_decode_raw_${index}`, Buffer.from(bytes), [
            {op: "VarInt Decode", args: []},
        ]);
    }

    for (const [index, value] of QUOTED_PRINTABLE.entries()) {
        addCase(`quoted_printable_${index}`, value, [
            {op: "From Quoted Printable", args: []},
        ]);
    }
}
