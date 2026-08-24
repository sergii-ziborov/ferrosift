// Compression, slicing, and search.
//
// Only the inflate direction is byte-pinned. The inputs are standard DEFLATE
// streams produced by Node's own zlib — the CyberChef node build's deflate
// operations abort under Node 24 — so the reference's inflate output is pinned
// against a producer-independent stream. Compression-direction parity is
// interoperable rather than bit-identical, and is exercised by the round-trip
// tests plus the pinned Bzip2 payload in differential.json.
import {deflateRawSync, deflateSync, gzipSync} from "node:zlib";

export function add({addCase, randomBytes}) {
    for (const length of [0, 1, 32, 200]) {
        const raw = randomBytes(length);
        addCase(`gunzip_${length}`, gzipSync(raw), [{op: "Gunzip", args: []}]);
        addCase(`zlib_inflate_${length}`, deflateSync(raw), [
            {op: "Zlib Inflate", args: [0, 0, "Adaptive", false, false]},
        ]);
        addCase(`raw_inflate_${length}`, deflateRawSync(raw), [
            {op: "Raw Inflate", args: [0, 0, "Adaptive", false, false]},
        ]);
    }

    // Data slicing and text head.
    for (const [start, length, name] of [[0, 4, "a"], [2, 3, "b"], [3, 0, "c"]]) {
        addCase(`take_bytes_${name}`, randomBytes(12), [
            {op: "Take bytes", args: [start, length, false]},
        ]);
        addCase(`drop_bytes_${name}`, randomBytes(12), [
            {op: "Drop bytes", args: [start, length, false]},
        ]);
    }
    for (const n of [1, 2, 3, -1]) {
        addCase(`head_${n}`, "l1\nl2\nl3\nl4\nl5", [{op: "Head", args: ["Line feed", n]}]);
    }
    addCase("find_replace_simple", "foo\tbar foo baz", [
        {
            op: "Find / Replace",
            args: [{option: "Simple string", string: "foo"}, "X", true, false, true, false],
        },
    ]);
    addCase("find_replace_regex", "a1b2c3d4", [
        {
            op: "Find / Replace",
            args: [{option: "Regex", string: "[0-9]"}, "#", true, false, true, false],
        },
    ]);
    addCase("strings_ascii", "\u0000\u0000Hello World\u0000AB\u0000secret_value", [
        {op: "Strings", args: ["Single byte", 4, "All printable chars (A)", false, false, false]},
    ]);
}
