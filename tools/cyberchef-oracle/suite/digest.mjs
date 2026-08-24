// Digests, MACs, and the inflate paths that feed them.

export const digestCases = [
    {
        name: "gunzip_hello",
        input: {
            kind: "bytes",
            hex: "1f8b080000000000000acb48cdc9c957484b2d2aca070084a9e47c0b000000",
        },
        recipe: [{op: "Gunzip", args: []}],
    },
    {
        name: "md5_hello",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [{op: "MD5", args: []}],
    },
    {
        name: "sha1_hello",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [{op: "SHA1", args: [80]}],
    },
    {
        name: "sha2_256_hello",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [{op: "SHA2", args: ["256", 64, 160]}],
    },
    {
        name: "hmac_sha256_utf8_key",
        input: {kind: "bytes", hex: "68656c6c6f"},
        recipe: [
            {
                op: "HMAC",
                args: [{option: "UTF8", string: "key"}, "SHA256"],
            },
        ],
    },
    {
        name: "zlib_inflate_hello",
        // Pinned CyberChef Zlib Deflate of "hello" (Dynamic Huffman Coding).
        input: {kind: "bytes", hex: "789c0580b105000000c3ae2d1d42fedfe20303062c0215"},
        recipe: [{op: "Zlib Inflate", args: [0, 0, "Adaptive", false, false]}],
    },
];
