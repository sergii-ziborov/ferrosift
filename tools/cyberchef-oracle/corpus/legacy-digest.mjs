// MD2, MD4, RIPEMD, SM3, and Whirlpool — the digests outside the SHA family.
//
// Lengths straddle each function's block boundary, which for MD2 is 16 bytes
// and for the rest 64, because that is where padding implementations diverge.

const LENGTHS = [0, 1, 15, 16, 17, 55, 56, 63, 64, 65, 119, 120, 200];

export function add({addCase, randomBytes}) {
    for (const length of LENGTHS) {
        const raw = randomBytes(length);
        addCase(`md2_${length}`, raw, [{op: "MD2", args: [18]}]);
        addCase(`md4_${length}`, raw, [{op: "MD4", args: []}]);
        addCase(`sm3_${length}`, raw, [{op: "SM3", args: [256, 64]}]);
        addCase(`whirlpool_${length}`, raw, [{op: "Whirlpool", args: ["Whirlpool", 10]}]);
        for (const size of ["128", "160", "256", "320"]) {
            addCase(`ripemd${size}_${length}`, raw, [{op: "RIPEMD", args: [size]}]);
        }
    }
}
