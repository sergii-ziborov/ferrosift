// BLAKE3, at lengths and inputs that cross the structures it is built from.
//
// The interesting question here is not whether the compression function is
// right -- one published algorithm, one answer -- but whether the port takes
// the same *shape* of output. Three boundaries decide that, and a port can be
// wrong at each one while passing everything either side of it:
//
//   * The output is an extendable stream, not a fixed digest. Past sixty-four
//     bytes it comes from a second block of the stream, so a port that stops at
//     one block agrees on every short length and fails past it.
//   * The input is chunked at a thousand and twenty-four bytes, and past one
//     chunk the chunks are combined through a tree rather than a chain. Inputs
//     on both sides of one and two chunks are here for that.
//   * Keyed BLAKE3 replaces the initial vector rather than prepending the key,
//     so a port that hashes the key first agrees with nothing.
//
// The key is read with the same rule as the input, which is not UTF-8: a
// character under two hundred and fifty-six contributes one byte. So a
// thirty-two byte key is thirty-two such characters, and the reference measures
// after the conversion rather than before.

const SIZES = [1, 2, 16, 31, 32, 33, 64, 65, 128, 1000];

const INPUTS = [
    "",
    "a",
    "abc",
    "The quick brown fox jumps over the lazy dog",
    // Non-ASCII, where the input conversion takes the code-unit branch and
    // yields one byte rather than the two UTF-8 would.
    " ÿ",
];

// Two thirty-two byte keys: one that is thirty-two characters of ASCII, and one
// whose characters are all above the ASCII range and still one byte each.
const KEYS = [
    "0123456789abcdef0123456789abcdef",
    "ÿ".repeat(32),
];

export async function add({addCase}) {
    let index = 0;
    for (const size of SIZES) {
        for (const input of INPUTS) {
            addCase(`blake3_${index}`, input, [{op: "BLAKE3", args: [size, ""]}]);
            index += 1;
        }
    }

    index = 0;
    for (const key of KEYS) {
        for (const size of [16, 32, 64, 65]) {
            for (const input of ["", "abc"]) {
                addCase(`blake3_keyed_${index}`, input, [{op: "BLAKE3", args: [size, key]}]);
                index += 1;
            }
        }
    }

    // Around the chunk boundary, where one chunk becomes a tree of them.
    index = 0;
    for (const length of [1023, 1024, 1025, 2047, 2048, 2049, 4096]) {
        addCase(`blake3_chunk_${index}`, "x".repeat(length), [{op: "BLAKE3", args: [32, ""]}]);
        index += 1;
    }
}
