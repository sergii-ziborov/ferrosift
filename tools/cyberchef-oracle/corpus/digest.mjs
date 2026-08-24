// XOR schemes and the digest / MAC family.
//
// The hash lengths straddle the SHA block boundaries (55, 64, 120) because
// that is where padding implementations diverge.

export function add({addCase, randomBytes}) {
    for (const scheme of ["Standard", "Input differential", "Output differential", "Cascade"]) {
        for (const length of [0, 1, 5, 16]) {
            addCase(`xor_${scheme.split(" ")[0].toLowerCase()}_${length}`, randomBytes(length), [
                {op: "XOR", args: [{option: "Hex", string: "3f"}, scheme, false]},
            ]);
        }
    }
    for (const length of [4, 9]) {
        addCase(`xor_null_${length}`, randomBytes(length), [
            {op: "XOR", args: [{option: "Hex", string: "01"}, "Standard", true]},
        ]);
        addCase(`xor_utf8key_${length}`, randomBytes(length), [
            {op: "XOR", args: [{option: "UTF8", string: "key"}, "Standard", false]},
        ]);
    }
    // XOR Brute Force emits a deterministic multi-line report; pin it directly.
    for (const [name, hex] of [["a", "1f001a1b00"], ["b", "48656c6c6f"]]) {
        addCase(`xor_brute_${name}`, Buffer.from(hex, "hex"), [
            {op: "XOR Brute Force", args: [1, 100, 0, "Standard", false, true, false, ""]},
        ]);
    }

    for (const length of [0, 1, 16, 55, 64, 120]) {
        const raw = randomBytes(length);
        addCase(`md5_${length}`, raw, [{op: "MD5", args: []}]);
        addCase(`sha1_${length}`, raw, [{op: "SHA1", args: [80]}]);
        addCase(`sha2_256_${length}`, raw, [{op: "SHA2", args: ["256", 64, 160]}]);
        addCase(`sha2_512_${length}`, raw, [{op: "SHA2", args: ["512", 64, 160]}]);
        addCase(`sha3_256_${length}`, raw, [{op: "SHA3", args: ["256"]}]);
        addCase(`sha3_512_${length}`, raw, [{op: "SHA3", args: ["512"]}]);
    }
    for (const hasher of ["MD5", "SHA1", "SHA256", "SHA512"]) {
        addCase(`hmac_${hasher.toLowerCase()}`, randomBytes(20), [
            {op: "HMAC", args: [{option: "UTF8", string: "ferro-key"}, hasher]},
        ]);
    }
}
