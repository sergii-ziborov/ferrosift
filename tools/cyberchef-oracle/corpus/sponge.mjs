// The Keccak sponge outside SHA-3: original Keccak, and SHAKE.
//
// Lengths straddle each variant's rate — the number of bytes absorbed per
// permutation — because that is where a padding implementation diverges from
// the specification without diverging on short inputs. Keccak-512 absorbs 72
// bytes, Keccak-256 absorbs 136, so 71/72/73 and 135/136/137 are each one
// short of, exactly at, and one past a block.
//
// SHAKE is sampled at output lengths shorter and longer than its own rate, so
// the squeeze phase is exercised at more than one permutation.

export function add({addCase, randomBytes}) {
    for (const length of [0, 1, 16, 71, 72, 73, 135, 136, 137]) {
        const raw = randomBytes(length);
        for (const size of ["512", "256"]) {
            addCase(`keccak_${size}_${length}`, raw, [{op: "Keccak", args: [size]}]);
        }
    }
    for (const length of [0, 5, 64]) {
        const raw = randomBytes(length);
        for (const size of ["384", "224"]) {
            addCase(`keccak_${size}_${length}`, raw, [{op: "Keccak", args: [size]}]);
        }
    }

    for (const length of [0, 1, 32, 168]) {
        const raw = randomBytes(length);
        for (const capacity of ["256", "128"]) {
            for (const size of [8, 512, 2048]) {
                addCase(`shake_${capacity}_${size}_${length}`, raw, [
                    {op: "Shake", args: [capacity, size]},
                ]);
            }
        }
    }
}
