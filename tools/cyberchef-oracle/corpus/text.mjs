// URL, HTML entity, ROT13, and hexdump: text transforms over printable ASCII.

export async function add({addCase, bakeString, cases, randomAscii, randomBytes}) {
    for (const length of [0, 1, 4, 8, 16, 24]) {
        const raw = randomBytes(length);
        addCase(`url_encode_${length}`, raw, [{op: "URL Encode", args: [false]}]);
        addCase(`url_encode_all_${length}`, raw, [{op: "URL Encode", args: [true]}]);
        const encoded = await bakeString(raw, [{op: "URL Encode", args: [false]}]);
        addCase(`url_decode_${length}`, encoded, [{op: "URL Decode", args: [true]}]);
    }
    // URL decode legacy-fallback and unicode-escape paths.
    for (const value of ["%E0%A4%A", "%FF%FE", "%u0413%u0414", "100%", "a+b%2Bc", "%C3%28"]) {
        addCase(`url_decode_edge_${cases.length}`, value, [{op: "URL Decode", args: [true]}]);
    }

    for (const length of [0, 4, 12, 24]) {
        // To/From HTML Entity are text-to-text; feed printable text, not bytes.
        // Named-entity encoding is a documented subset divergence (FerroSift emits
        // the classic entity set, CyberChef the full HTML5 table), so the corpus
        // pins the numeric-entity encode path and decoding of numeric entities.
        const raw = randomAscii(length).toString("latin1");
        const numeric = {op: "To HTML Entity", args: [true, "Numeric entities"]};
        addCase(`html_numeric_${length}`, raw, [numeric]);
        const encoded = await bakeString(raw, [numeric]);
        addCase(`html_from_numeric_${length}`, encoded, [{op: "From HTML Entity", args: []}]);
    }

    for (const length of [0, 5, 13, 26]) {
        addCase(`rot13_${length}`, randomAscii(length), [
            {op: "ROT13", args: [true, true, false, 13]},
        ]);
    }

    // Hexdump (round trip and width/flag variants).
    for (const length of [0, 1, 8, 16, 31]) {
        const raw = randomBytes(length);
        addCase(`hexdump_${length}`, raw, [
            {op: "To Hexdump", args: [16, false, false, false]},
            {op: "From Hexdump", args: []},
        ]);
        addCase(`hexdump_upper_${length}`, raw, [
            {op: "To Hexdump", args: [8, true, true, false]},
        ]);
    }
}
