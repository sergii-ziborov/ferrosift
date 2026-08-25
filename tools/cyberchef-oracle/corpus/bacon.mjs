// Bacon's cipher, header stripping, and the comment no-op.
//
// The Bacon cases cover all four ways of writing the two symbols, both
// alphabets, and both the keep and invert flags, because each changes the
// output at a different stage: inversion runs before the non-symbol characters
// are discarded, so a literal `0` in the input is flipped along with the code.
//
// One decode case feeds the literal text "undefined" through the first-letter
// translation. That translation has no entry in the reference's table of
// characters to strip, so `String.replace` receives `undefined`, coerces it to
// the string, and removes the first occurrence. Pinning it is the only way to
// keep a port from quietly "fixing" it.

const IPV4 = Buffer.from([
    0x45, 0x00, 0x00, 0x2c, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06, 0x00, 0x00,
    0xc0, 0xa8, 0x00, 0x01, 0xc0, 0xa8, 0x00, 0x02,
    0xde, 0xad, 0xbe, 0xef,
]);

// IHL 6: a header with one 32-bit option word before the payload.
const IPV4_OPTIONS = Buffer.from([
    0x46, 0x00, 0x00, 0x30, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06, 0x00, 0x00,
    0xc0, 0xa8, 0x00, 0x01, 0xc0, 0xa8, 0x00, 0x02, 0x01, 0x02, 0x03, 0x04,
    0xca, 0xfe,
]);

const TCP = Buffer.from([
    0x00, 0x50, 0x1f, 0x90, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x50, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x70, 0x61, 0x79,
]);

// Data offset 6: twenty bytes plus one option word.
const TCP_OPTIONS = Buffer.from([
    0x00, 0x50, 0x1f, 0x90, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x60, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x04, 0x05, 0xb4,
    0x6c, 0x6f, 0x61, 0x64,
]);

const UDP = Buffer.from([
    0x1f, 0x90, 0x00, 0x35, 0x00, 0x0c, 0x00, 0x00, 0x64, 0x61, 0x74, 0x61,
]);

const ALPHABETS = ["Standard (I=J and U=V)", "Complete"];

export function add({addCase}) {
    for (const alphabet of ALPHABETS) {
        const tag = alphabet.startsWith("Standard") ? "std" : "full";
        for (const translation of ["0/1", "A/B"]) {
            const symbol = translation === "0/1" ? "01" : "ab";
            for (const keep of [false, true]) {
                for (const invert of [false, true]) {
                    const name = `bacon_encode_${tag}_${symbol}_${keep ? "keep" : "drop"}_${invert ? "inv" : "std"}`;
                    addCase(name, "Attack at dawn, IJUV 01!", [
                        {op: "Bacon Cipher Encode", args: [alphabet, translation, keep, invert]},
                    ]);
                }
            }
        }
    }

    const decodeInputs = {
        "0/1": "00000 00001 00010 11001",
        "A/B": "AAAAA AAAAB AAABA BBAAB",
        "Case": "aBcDe FgHiJ kLmNo PqRsT",
        "A-M/N-Z first letter": "never a zebra, only apples now — ever",
    };
    for (const alphabet of ALPHABETS) {
        const tag = alphabet.startsWith("Standard") ? "std" : "full";
        for (const [translation, text] of Object.entries(decodeInputs)) {
            const symbol = translation.replace(/[^A-Za-z]/g, "").toLowerCase().slice(0, 6);
            for (const invert of [false, true]) {
                addCase(`bacon_decode_${tag}_${symbol}_${invert ? "inv" : "std"}`, text, [
                    {op: "Bacon Cipher Decode", args: [alphabet, translation, invert]},
                ]);
            }
        }
    }
    // The `undefined` artefact: only the first-letter translation reaches it.
    addCase("bacon_decode_undefined", "never undefined a zebra only apples now", [
        {op: "Bacon Cipher Decode", args: ["Complete", "A-M/N-Z first letter", false]},
    ]);

    addCase("strip_ipv4_basic", IPV4, [{op: "Strip IPv4 header", args: []}]);
    addCase("strip_ipv4_options", IPV4_OPTIONS, [{op: "Strip IPv4 header", args: []}]);
    addCase("strip_tcp_basic", TCP, [{op: "Strip TCP header", args: []}]);
    addCase("strip_tcp_options", TCP_OPTIONS, [{op: "Strip TCP header", args: []}]);
    addCase("strip_udp_basic", UDP, [{op: "Strip UDP header", args: []}]);
    // A datagram that is only a header: the payload is empty, not an error.
    addCase("strip_udp_empty", UDP.subarray(0, 8), [{op: "Strip UDP header", args: []}]);

    // `Comment` is absent here on purpose. It is flow control, and the
    // reference's Node build does not expose flow-control operations at all,
    // so there is nothing to bake against. It is pinned in
    // `conformance_fork.rs` alongside Fork and Merge, and recorded in
    // `docs/compatibility/exemptions.json` so the coverage gate agrees.
}
