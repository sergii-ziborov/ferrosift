// COBS framing, Base62, and ASN.1 object identifiers.
//
// Three encodings that each hinge on something a plausible port gets wrong:
// where COBS splits a maximal run, whether Base62 keeps leading zero bytes,
// and how the first two arcs of an object identifier share one value.

const A62 = "0-9A-Za-z";

/** Byte strings chosen for where COBS's block boundaries fall. */
const COBS_INPUTS = [
    [],
    [1, 2, 3],
    // Every arrangement of zeros that changes which branch runs.
    [0],
    [0, 0],
    [0, 0, 0],
    [1, 0, 3],
    [1, 0, 0, 2],
    [0, 1, 0],
    [1, 2, 3, 0],
    [0, 1, 2, 3],
    // 253/254/255 bracket the point where a run stops fitting one length byte,
    // which is the boundary two reasonable implementations disagree about.
    Array.from({length: 253}, (_, index) => (index % 254) + 1),
    Array.from({length: 254}, (_, index) => (index % 254) + 1),
    Array.from({length: 255}, (_, index) => (index % 254) + 1),
    Array.from({length: 300}, (_, index) => (index % 254) + 1),
    // A zero before a maximal run: the decoder's two 0xFF paths differ by
    // whether they emit a separator, and only this reaches the second one.
    [1, 0, ...Array.from({length: 300}, (_, index) => (index % 254) + 1)],
    // A maximal run followed by a zero.
    [...Array.from({length: 300}, (_, index) => (index % 254) + 1), 0, 7],
];

/** Frames fed straight to the decoder, including ones no encoder produces. */
const COBS_FRAMES = [
    [1],
    [2],
    [1, 1],
    [1, 1, 1],
    [4, 1, 2, 3],
    [2, 1, 2, 3],
    // Promises four bytes and supplies two: the reference returns what it has.
    [5, 1, 2],
    [255],
    [255, 1, 2, 3],
];

const BASE62_TEXT = ["A", "hello", "The quick brown fox", "z".repeat(40)];

/** Byte strings where the leading-zero question and the zero value both bite. */
const BASE62_BYTES = [
    "00",
    "0000",
    "ff",
    "ffff",
    "0001",
    "0100",
    "00ff00ff",
    "deadbeef",
    "0102030405060708090a0b0c0d0e0f",
];

const BASE62_STRINGS = ["0", "1", "zzzz", "0000", "Zz09", "00A", "A00", "aA0".repeat(20)];

const OIDS = [
    // The identifiers anyone actually meets.
    "1.2.840.113549.1.1.11",
    "2.5.4.3",
    "2.16.840.1.101.3.4.2.1",
    "1.2.840.10045.4.3.2",
    "1.3.6.1.4.1.311.21.20",
    "2.16.840.1.113730.1.13",
    // The first-pair arithmetic, at each boundary of the 40-arc split.
    "0.0",
    "0.39",
    "0.40",
    "1.39",
    "1.40",
    "2.40",
    "1.2",
    "1.3",
    "2.0",
    "2.16",
    // First pairs past one byte, where the reference emits hex that no decoder
    // reads back. Pinned because that is what it emits.
    "2.999",
    "2.100.3",
    "127.127",
    "255.255",
    "3.1.1",
    "9999999999.1",
    // The first pair goes through JavaScript doubles while later arcs go
    // through an exact big integer, so these lose precision and the ones below
    // them do not. Pinned because an implementation that computed the pair
    // exactly would be arithmetically better and byte-wrong.
    "9007199254740993.1",
    "18014398509481984.0",
    "99999999999999999999.1",
    "12345678901234567890.7",
    // Base-128 continuation at each power of the boundary.
    "1.2.127",
    "1.2.128",
    "1.2.16383",
    "1.2.16384",
    "1.2.3.4294967296",
    // Past every fixed width, which is why this needs arbitrary precision.
    "2.25.223663413560170236103021352304496384762",
    "1.2.340282366920938463463374607431768211456",
    // Leading zeros and empty trailing arcs, both read as decimal.
    "01.02",
    "1.2.03",
    "1.2.0",
    "1.2.",
    "0.0.0",
];

/** Hex fed straight to the decoder, including shapes no encoder produces. */
const OID_HEX = [
    "2a864886f70d01010b",
    "550403",
    "608648016503040201",
    "2a8648ce3d040302",
    "0000",
    "2b",
    "01",
    "0a",
    "63",
    "7f",
    "80",
    "ff",
    "8837",
    "8000",
    "807f",
    "819c",
    "8180808000",
    "ffffffff7f",
    // An odd trailing nibble, read as a byte on its own.
    "2a8",
    "2a86488",
    "0",
    // A continuation that never terminates: dropped, not reported.
    "2a86",
    "8080",
    "2a8080",
    // Whitespace, stripped before anything else happens.
    "2a 86 48",
    "2a\t86\n48",
    // Upper case.
    "2A864886F70D01010B",
];

export function add({addCase}) {
    for (const [index, bytes] of COBS_INPUTS.entries()) {
        // One recipe rather than two cases, because the corpus pins every
        // prefix: this checks the frame *and* that the decoder reads back what
        // the encoder wrote. A frame this decoder cannot read is a frame this
        // encoder should not have written, and one case now says both.
        addCase(`cobs_round_trip_${index}`, Buffer.from(bytes), [
            {op: "To COBS", args: []},
            {op: "From COBS", args: []},
        ]);
    }
    for (const [index, bytes] of COBS_FRAMES.entries()) {
        addCase(`cobs_decode_${index}`, Buffer.from(bytes), [{op: "From COBS", args: []}]);
    }

    // Passed as bytes, not text: To Base62 reads an ArrayBuffer, so the
    // reference would UTF-8 encode a string anyway. Saying so here keeps the
    // case honest about what the operation actually consumes.
    for (const [index, text] of BASE62_TEXT.entries()) {
        addCase(`base62_encode_text_${index}`, Buffer.from(text, "utf8"), [
            {op: "To Base62", args: [A62]},
        ]);
    }
    for (const [index, hex] of BASE62_BYTES.entries()) {
        addCase(`base62_encode_bytes_${index}`, Buffer.from(hex, "hex"), [
            {op: "To Base62", args: [A62]},
        ]);
    }
    for (const [index, text] of BASE62_STRINGS.entries()) {
        addCase(`base62_decode_${index}`, text, [{op: "From Base62", args: [A62]}]);
    }
    // Characters outside the alphabet are dropped rather than refused.
    addCase("base62_decode_filtered", "he!!o", [{op: "From Base62", args: [A62]}]);
    addCase("base62_decode_all_filtered", "-!-", [{op: "From Base62", args: [A62]}]);
    // A reordered alphabet changes every digit, so this catches a port that
    // hard-codes the default one.
    addCase("base62_encode_reordered", Buffer.from("hello", "utf8"), [
        {op: "To Base62", args: ["a-z0-9A-Z"]},
    ]);
    addCase("base62_decode_reordered", "gXh", [{op: "From Base62", args: ["a-z0-9A-Z"]}]);
    // Empty input short-circuits before the alphabet is looked at, so a
    // malformed alphabet goes unreported here.
    addCase("base62_encode_empty_bad_alphabet", Buffer.alloc(0), [
        {op: "To Base62", args: ["!"]},
    ]);
    addCase("base62_decode_empty_bad_alphabet", "", [{op: "From Base62", args: ["!"]}]);

    for (const [index, oid] of OIDS.entries()) {
        addCase(`oid_encode_${index}`, oid, [{op: "Object Identifier to Hex", args: []}]);
    }
    for (const [index, hex] of OID_HEX.entries()) {
        addCase(`oid_decode_${index}`, hex, [{op: "Hex to Object Identifier", args: []}]);
    }
    // The round trip, which holds only while the first pair fits in one byte.
    for (const oid of ["1.2.840.113549.1.1.11", "2.5.4.3", "1.3.6.1.4.1.311.21.20"]) {
        addCase(`oid_round_trip_${oid}`, oid, [
            {op: "Object Identifier to Hex", args: []},
            {op: "Hex to Object Identifier", args: []},
        ]);
    }
}
