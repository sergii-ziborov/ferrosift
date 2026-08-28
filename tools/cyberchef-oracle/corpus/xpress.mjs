// XPRESS (MS-XCA), both variants, which 11.4 introduced.
//
// Every other decoder family here samples raw bytes and runs them through the
// reference's own *encoder* to get a canonical input. XPRESS has no encoder:
// the reference reads these formats and Windows writes them. So the inputs are
// built here, by two small encoders that exist only to reach branches -- and
// what makes them evidence rather than a second opinion is that the reference
// decompresses them and the fixture records what it produced. An encoder bug
// cannot invent a passing case: a stream the reference refuses fails to bake
// and the case is dropped, which the generator reports.
//
// The branches worth reaching are the ones a port gets wrong quietly. In the
// plain variant that is the shared nibble, where one byte carries the length
// extension of two matches that may be far apart. In the Huffman variant it is
// the byte-interleaved raw length, which is read from the same cursor the bit
// reader refills from and so shifts the whole stream's alignment, and the
// end-of-data symbol, which is an ordinary three-byte match anywhere except at
// exactly the declared length.

/* ---------------- plain LZ77 ---------------- */

/**
 * Encodes items into an XPRESS plain-LZ77 stream.
 *
 * An item is `{lit}` for a literal byte or `{off, len}` for a match. Flag
 * groups are 32 bits written before the items they describe, tested from bit
 * 31 down, and the stream ends with a set flag that has no input behind it.
 */
function encodePlain(items) {
    const out = [];
    let flagAt = -1;
    let bit = -1;
    let flags = 0;
    // Where the half-used nibble byte sits, exactly as the decoder tracks it.
    let pendingAt = -1;

    const writeFlags = () => {
        if (flagAt < 0) return;
        out[flagAt] = flags & 0xff;
        out[flagAt + 1] = (flags >>> 8) & 0xff;
        out[flagAt + 2] = (flags >>> 16) & 0xff;
        out[flagAt + 3] = (flags >>> 24) & 0xff;
    };
    const startGroup = () => {
        writeFlags();
        flagAt = out.length;
        out.push(0, 0, 0, 0);
        flags = 0;
        bit = 31;
    };

    startGroup();
    for (const item of items) {
        if (bit < 0) startGroup();
        if (item.lit !== undefined) {
            out.push(item.lit & 0xff);
            bit -= 1;
            continue;
        }

        flags |= 1 << bit;
        bit -= 1;
        const {off, len} = item;
        const low = len - 3 >= 7 ? 7 : len - 3;
        const word = ((off - 1) << 3) | low;
        out.push(word & 0xff, (word >>> 8) & 0xff);
        if (low !== 7) continue;

        // The extension nibble. The first match to need one writes a fresh
        // byte and leaves its high nibble for whichever match needs one next,
        // which is the part that cannot be modelled as a per-match field.
        const nibble = pickNibble(len);
        if (pendingAt < 0) {
            pendingAt = out.length;
            out.push(nibble & 0x0f);
        } else {
            out[pendingAt] |= (nibble & 0x0f) << 4;
            pendingAt = -1;
        }
        if (nibble === 15) out.push(...rawLengthBytes(len));
    }

    // The last group is padded with set bits, so the decoder meets a match
    // flag with nothing behind it and stops. A group that filled exactly gets
    // an empty one after it, for the same reason.
    if (bit < 0) startGroup();
    for (let b = bit; b >= 0; b--) flags |= 1 << b;
    writeFlags();
    return Buffer.from(out);
}

/** Which nibble encodes this length, 15 meaning "read a raw length". */
function pickNibble(len) {
    return len - 10 <= 14 ? len - 10 : 15;
}

/**
 * The raw-length bytes for a length the nibble could not hold.
 *
 * A byte below 255 means `len - 25`. Otherwise 255 introduces an LE16 of
 * `len - 3`, and an LE16 of zero introduces an LE32 of the same. The reference
 * refuses an escaped value below 22, because the short form could have said
 * it, so the two forms never overlap.
 */
function rawLengthBytes(len) {
    const short = len - 25;
    if (short >= 0 && short <= 254) return [short];
    const value = len - 3;
    if (value <= 0xffff && value !== 0) {
        return [255, value & 0xff, (value >>> 8) & 0xff];
    }
    return [
        255,
        0,
        0,
        value & 0xff,
        (value >>> 8) & 0xff,
        (value >>> 16) & 0xff,
        (value >>> 24) & 0xff,
    ];
}

/* ---------------- LZ77 + Huffman ---------------- */

const HUFFMAN_SYMBOLS = 512;
const TABLE_BITS = 15;

/**
 * Canonical codes for a code-length set, assigned exactly as the decoder
 * builds its table: in (length, symbol) order, most-significant bit first.
 *
 * Deriving the codes from the same walk the decoder does is what keeps the two
 * from drifting. A hand-assigned code table would be a second implementation
 * of the same rule, and the case would then be evidence about the encoder.
 */
function canonicalCodes(lengths) {
    const codes = new Array(HUFFMAN_SYMBOLS).fill(0);
    let filled = 0;
    for (let length = 1; length <= TABLE_BITS; length++) {
        for (let symbol = 0; symbol < HUFFMAN_SYMBOLS; symbol++) {
            if (lengths[symbol] !== length) continue;
            codes[symbol] = filled >>> (TABLE_BITS - length);
            filled += 1 << (TABLE_BITS - length);
        }
    }
    if (filled !== 1 << TABLE_BITS) {
        throw new Error(`code lengths fill ${filled} of ${1 << TABLE_BITS} table entries`);
    }
    return codes;
}

/** The 256-byte header: two four-bit lengths per byte, even one low. */
function huffmanHeader(lengths) {
    const header = [];
    for (let i = 0; i < 256; i++) {
        header.push((lengths[i * 2] & 0x0f) | ((lengths[i * 2 + 1] & 0x0f) << 4));
    }
    return header;
}

/**
 * Encodes items into an XPRESS LZ77+Huffman stream.
 *
 * Two passes, because the raw-length bytes are not part of the bit stream but
 * sit *inside* the byte stream at whatever position the bit reader's cursor
 * has reached. The first pass produces the bits, which do not depend on where
 * those bytes land; the second walks the decoder's own refill schedule and
 * emits words and raw bytes in the order it will read them.
 *
 * An item is `{lit}`, `{end: true}`, or `{off, len}`.
 */
function encodeHuffman(lengths, items) {
    const codes = canonicalCodes(lengths);
    const bits = [];
    const plan = [];

    const pushBits = (value, width) => {
        for (let b = width - 1; b >= 0; b--) bits.push((value >>> b) & 1);
    };
    // A symbol the code set does not assign has no code, and emitting zero
    // bits for it would produce a stream that decodes to something else
    // entirely. That case would then be dropped as a bake failure, which reads
    // as "the reference refused this" rather than "the encoder asked for a
    // symbol that is not in the table".
    const emit = symbol => {
        if (!lengths[symbol]) {
            throw new Error(`symbol ${symbol} has no code in this length set`);
        }
        pushBits(codes[symbol], lengths[symbol]);
    };

    for (const item of items) {
        if (item.lit !== undefined) {
            emit(item.lit);
            plan.push({symbol: item.lit, extra: 0, raw: null});
            continue;
        }
        if (item.end) {
            emit(256);
            plan.push({symbol: 256, extra: 0, raw: null});
            continue;
        }

        const {off, len} = item;
        let extra = 0;
        while (1 << (extra + 1) <= off) extra += 1;
        const lengthCode = len - 3 <= 14 ? len - 3 : 15;
        const symbol = 256 + (extra << 4) + lengthCode;
        emit(symbol);
        // The offset's low bits follow the symbol, after any raw length.
        const raw = lengthCode === 15 ? huffmanRawLength(len) : null;
        plan.push({symbol, extra, raw});
        if (extra > 0) pushBits(off - (1 << extra), extra);
    }

    const out = huffmanHeader(lengths);
    let position = 0;
    let available = 0;
    const emitWord = () => {
        let word = 0;
        for (let k = 0; k < 16; k++) {
            word = (word << 1) | (position < bits.length ? bits[position++] : 0);
        }
        out.push(word & 0xff, (word >>> 8) & 0xff);
        available += 16;
    };

    while (available < 32) emitWord();
    for (const step of plan) {
        while (available < TABLE_BITS) emitWord();
        available -= lengths[step.symbol];
        if (step.raw) out.push(...step.raw);
        while (available < step.extra) emitWord();
        available -= step.extra;
    }
    // Whatever the decoder still wants after the end symbol it will not ask
    // for, but a stream that ends exactly at a word boundary leaves it nothing
    // to refill from if it does, so one spare word is cheap insurance.
    emitWord();
    return Buffer.from(out);
}

/** The Huffman variant's raw length, which has base 18 rather than 25. */
function huffmanRawLength(len) {
    const short = len - 18;
    if (short >= 0 && short <= 254) return [short];
    const value = len - 3;
    if (value <= 0xffff && value !== 0) {
        return [255, value & 0xff, (value >>> 8) & 0xff];
    }
    return [
        255,
        0,
        0,
        value & 0xff,
        (value >>> 8) & 0xff,
        (value >>> 16) & 0xff,
        (value >>> 24) & 0xff,
    ];
}

/** Every symbol nine bits wide, which fills the table exactly. */
function uniformLengths() {
    return new Array(HUFFMAN_SYMBOLS).fill(9);
}

/**
 * A deliberately lopsided code set, to exercise the canonical ordering.
 *
 * One symbol per length from 1 to 14 plus two at 15, which is the smallest
 * shape whose codes are all different widths. A table where every code is the
 * same width would pass with the (length, symbol) ordering ignored.
 */
function skewedLengths() {
    const lengths = new Array(HUFFMAN_SYMBOLS).fill(0);
    const order = [
        0x41, 0x42, 0x43, 256, 257, 258, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b,
    ];
    order.forEach((symbol, index) => {
        lengths[symbol] = index + 1;
    });
    lengths[0x4c] = 15;
    lengths[0x4d] = 15;
    return lengths;
}

/** What a stream of items decompresses to, for choosing a declared size. */
function expand(items) {
    const out = [];
    for (const item of items) {
        if (item.lit !== undefined) {
            out.push(item.lit);
            continue;
        }
        if (item.end) continue;
        const start = out.length - item.off;
        for (let j = 0; j < item.len; j++) out.push(out[start + j]);
    }
    return out;
}

const LITERALS = [...Buffer.from("The quick brown fox jumps over the lazy dog. ")];

export async function add({addCase}) {
    /* ---- plain ---- */

    const plain = (name, items) =>
        addCase(`xpress_${name}`, encodePlain(items), [{op: "XPRESS Decompress", args: []}]);

    // Literal-only streams either side of a flag group's 32 items.
    for (const size of [0, 1, 5, 31, 32, 33, 64, 100]) {
        plain(
            `literals_${size}`,
            Array.from({length: size}, (unused, index) => ({lit: LITERALS[index % LITERALS.length]})),
        );
    }

    const prelude = LITERALS.slice(0, 40).map(lit => ({lit}));

    // Every match length the low three bits can hold, with no nibble.
    for (let len = 3; len <= 9; len++) {
        plain(`match_short_${len}`, [...prelude, {off: 10, len}]);
    }
    // Every nibble below the escape.
    for (let nibble = 0; nibble <= 14; nibble++) {
        plain(`match_nibble_${nibble}`, [...prelude, {off: 12, len: nibble + 10}]);
    }
    // The escape's short form: a byte meaning `len - 25`.
    for (const len of [25, 26, 60, 279]) {
        plain(`match_raw_short_${len}`, [...prelude, {off: 8, len}]);
    }
    // The escape's LE16 form, whose smallest legal value is 22.
    for (const len of [25 + 255, 1000, 5000]) {
        plain(`match_raw_wide_${len}`, [...prelude, {off: 8, len}]);
    }

    // Two nibble matches sharing one byte, adjacent and far apart. Getting the
    // pending byte wrong produces plausible output rather than an error.
    plain("shared_nibble_adjacent", [
        ...prelude,
        {off: 10, len: 12},
        {off: 20, len: 17},
    ]);
    plain("shared_nibble_separated", [
        ...prelude,
        {off: 10, len: 12},
        ...prelude,
        {lit: 0x21},
        {off: 30, len: 24},
    ]);
    // Three of them, so the pairing has to alternate rather than reset.
    plain("shared_nibble_three", [
        ...prelude,
        {off: 10, len: 11},
        {off: 15, len: 13},
        {off: 25, len: 19},
    ]);

    // A run pointing into the bytes it is producing.
    plain("overlap_distance_one", [{lit: 0x61}, {off: 1, len: 30}]);
    plain("overlap_distance_three", [{lit: 0x61}, {lit: 0x62}, {lit: 0x63}, {off: 3, len: 25}]);
    // The widest offset the format allows.
    plain("offset_maximum", [
        ...Array.from({length: 8192}, (unused, index) => ({lit: LITERALS[index % LITERALS.length]})),
        {off: 8192, len: 16},
    ]);
    // Matches straddling a flag group boundary.
    plain("across_flag_groups", [
        ...prelude,
        ...Array.from({length: 40}, (unused, index) =>
            index % 3 === 0 ? {off: 12, len: 5} : {lit: LITERALS[index % LITERALS.length]},
        ),
    ]);

    // Feeding the output onward, which is where the value model shows.
    addCase("xpress_then_hex", encodePlain([{lit: 0x61}, {off: 1, len: 10}]), [
        {op: "XPRESS Decompress", args: []},
        {op: "To Hex", args: ["Space", 0]},
    ]);

    /* ---- LZ77 + Huffman ---- */

    const huffman = (name, lengths, items) => {
        const size = expand(items).length;
        addCase(`xpress_huffman_${name}`, encodeHuffman(lengths, [...items, {end: true}]), [
            {op: "XPRESS LZ77+Huffman Decompress", args: [size]},
        ]);
    };

    const uniform = uniformLengths();
    for (const size of [1, 2, 17, 64, 300]) {
        huffman(
            `literals_${size}`,
            uniform,
            Array.from({length: size}, (unused, index) => ({lit: LITERALS[index % LITERALS.length]})),
        );
    }

    const seed = LITERALS.map(lit => ({lit}));
    // Every offset bit width the symbol space encodes, up to one that needs a
    // few kilobytes of window behind it.
    for (const width of [0, 1, 2, 3, 4, 8, 10]) {
        const window = 1 << width;
        huffman(
            `offset_width_${width}`,
            uniform,
            [
                ...Array.from({length: window + 4}, (unused, index) => ({
                    lit: LITERALS[index % LITERALS.length],
                })),
                {off: window, len: 6},
            ],
        );
    }
    // Every length the symbol's low nibble can hold, then the escape.
    for (let len = 3; len <= 17; len++) {
        huffman(`match_length_${len}`, uniform, [...seed, {off: 8, len}]);
    }
    for (const len of [18, 19, 100, 272, 273, 1000]) {
        huffman(`match_raw_${len}`, uniform, [...seed, ...seed, ...seed, {off: 8, len}]);
    }
    // Two escaped lengths in a row, which shifts the bit stream's byte
    // alignment twice and is where a decoder that treats the stream as
    // word-aligned comes apart.
    huffman("match_raw_twice", uniform, [
        ...seed,
        ...seed,
        {off: 8, len: 40},
        {off: 12, len: 60},
    ]);
    // A run pointing into itself.
    huffman("overlap_distance_one", uniform, [{lit: 0x61}, {off: 1, len: 30}]);

    // The end-of-data symbol away from the declared length, where it is an
    // ordinary match of three bytes at distance one.
    addCase(
        "xpress_huffman_end_symbol_midstream",
        encodeHuffman(uniformLengths(), [
            {lit: 0x61},
            {lit: 0x62},
            {end: true},
            {lit: 0x63},
            {end: true},
        ]),
        [{op: "XPRESS LZ77+Huffman Decompress", args: [6]}],
    );

    // A code set whose codes are all different widths.
    const skewed = skewedLengths();
    huffman("skewed_literals", skewed, [
        {lit: 0x41},
        {lit: 0x42},
        {lit: 0x43},
        {lit: 0x44},
        {lit: 0x4b},
        {lit: 0x4c},
        {lit: 0x4d},
        {lit: 0x41},
    ]);
    // The only two match symbols this set assigns, both at distance one: 257
    // and 258, which are lengths four and five. A wider match would need a
    // symbol the table does not carry.
    huffman("skewed_match", skewed, [
        {lit: 0x41},
        {lit: 0x42},
        {lit: 0x43},
        {lit: 0x44},
        {off: 1, len: 4},
        {off: 1, len: 5},
    ]);

    addCase(
        "xpress_huffman_then_hex",
        encodeHuffman(uniformLengths(), [{lit: 0x61}, {off: 1, len: 9}, {end: true}]),
        [
            {op: "XPRESS LZ77+Huffman Decompress", args: [10]},
            {op: "To Hex", args: ["Space", 0]},
        ],
    );
}
