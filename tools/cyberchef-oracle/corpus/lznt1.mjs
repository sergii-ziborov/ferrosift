// LZNT1, the compression behind `RtlDecompressBuffer`.
//
// CyberChef ships no LZNT1 compressor, so these streams are hand-built from
// the format rather than produced by a paired encoder. Each is a header word
// -- the low twelve bits one less than the block size, the top bit saying
// whether the block is compressed -- followed by the block.
//
// The back-references are what make the format worth pinning. The split
// between distance and length is not fixed: it widens as the block fills, so
// the same two bytes mean different things at different positions. A decoder
// that assumes a fixed split reads early blocks correctly and later ones as
// noise, which is exactly the bug a fixture catches and a smoke test does not.
//
// A run may also point into the bytes it is producing, which is how a repeat
// longer than the distance behind it is encoded -- copying the whole span at
// once would read bytes that do not exist yet.

const STREAMS = [
    // Uncompressed block: size word, then four literal bytes.
    "030061626364",
    // Compressed block, all literals: one flag byte of zeroes.
    "04800061626364",
    // Compressed block with a back-reference: literal `a`, then distance one
    // and length six, which unrolls into eight `a`s by reading its own output.
    "038002610400",
    // Two blocks in sequence, the second continuing after the first.
    "030061626364030065666768",
    // A zero size ends the stream, so the trailing bytes are never read.
    "0300616263640000ffffffff",
    // Empty input, and input too short to hold a header.
    "",
    "03",
    // Sixteen literals, which need *two* flag bytes: one flag byte covers
    // eight items, so a single one leaves the ninth byte being read as flags.
    "1180000011223344556677008899aabbccddeeff",
];

export function add({addCase}) {
    let index = 0;
    for (const hex of STREAMS) {
        addCase(`lznt1_${index++}`, Buffer.from(hex, "hex"), [
            {op: "LZNT1 Decompress", args: []},
        ]);
    }
}