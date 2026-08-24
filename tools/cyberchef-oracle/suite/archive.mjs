// Archive payloads and the extraction operations over them.

export const archiveCases = [
    {
        // Compress first so the fixture pins CyberChef ciphertext; FerroSift
        // only needs to match the final inflate (compress is interoperable,
        // not bit-identical). Differential runner still checks every prefix,
        // so this case is import+final only via a decompress-only recipe.
        name: "raw_inflate_cyberchef_payload",
        input: {
            kind: "bytes",
            hex: "0540c1090020105aa5696e814608031f6568eb3f8e82adc9fdc7d2794642dd06",
        },
        recipe: [{op: "Raw Inflate", args: [0, 0, "Adaptive", false, false]}],
    },
    {
        name: "bzip2_decompress_cyberchef_payload",
        input: {
            kind: "bytes",
            hex: "425a683931415926535911be4bc300000097804000010008000b23dc0020003100000843d469e93ca337a0ae9f201da61561fc5dc914e1424046f92f0c",
        },
        recipe: [{op: "Bzip2 Decompress", args: [false]}],
    },
    {
        name: "extract_mac_addresses_sorted_unique",
        input: {
            kind: "text",
            value: "Hosts aa:bb:cc:dd:ee:ff and AA-BB-CC-DD-EE-FF and aa:bb:cc:dd:ee:ff and 11:22:33:44:55:66",
        },
        recipe: [{op: "Extract MAC addresses", args: [true, true, true]}],
    },
    {
        name: "extract_hashes_sha1_length",
        input: {
            kind: "text",
            value: "md5 deadbeefcafebabe0123456789abcdef and sha1 0123456789abcdef0123456789abcdef01234567 and again 0123456789abcdef0123456789abcdef01234567",
        },
        recipe: [{op: "Extract hashes", args: [40, false, false]}],
    },
    {
        name: "extract_file_paths_win_unix",
        input: {
            kind: "text",
            value: "See C:\\Windows\\System32\\cmd.exe and /usr/bin/python3.11 and C:\\Temp\\file.txt",
        },
        recipe: [{op: "Extract file paths", args: [true, true, false, false, false]}],
    },
];
