// Reading a bcrypt hash apart, including the parts that are not a bcrypt hash.
//
// No bcrypt is computed: the operation counts characters and prints fields. So
// what is worth pinning is what the reference's library does with a sixty
// character string that is *not* a hash, because it does not check. Three
// behaviours follow from that, and each one is a place a tidier port differs:
//
//   * The rounds are read with `parseInt` and printed, never validated. No
//     third field prints `NaN`; a field of thirty digits prints in exponential
//     notation, because what `parseInt` returns is a double.
//   * The salt is the first twenty-nine characters taken by count, and the
//     password hash is `split(salt)[1]` -- what lies between the first two
//     occurrences of the salt, not "the rest". Twice twenty-nine fits inside
//     sixty, so sixty identical characters give an *empty* password hash.
//   * Only the total length is checked, and it is checked in UTF-16 code units.
//
// The refusals cannot be pinned here, because the reference throws and a bake
// that throws is a generation failure rather than a case. They are in
// tests/conformance_bcrypt.rs.

/** How long a bcrypt hash is, and the only thing the reference checks. */
const LENGTH = 60;

/** The prefix, padded to the length the reference demands. */
function sixty(prefix) {
    if (prefix.length > LENGTH) throw new Error(`${prefix} is longer than ${LENGTH}`);
    return prefix + "x".repeat(LENGTH - prefix.length);
}

// Real hashes, from the published bcrypt vectors, across the cost values and
// the three version tags in circulation.
const HASHES = [
    "$2a$04$5DCebwootqWMCp59ISrMJ.l4WvgHIVg17ZawDIrDM2IjlE64GDNQS",
    "$2a$06$DCq7YPn5Rq63x1Lad4cll.TV4S6ytwfsfvkgY8jIucDrjc8deX1s.",
    "$2a$06$If6bvum7DFjUnE9p2uDeDu0YHzrHM6tf.iqN8.yx.jNN1ILEf7h0i",
    "$2a$08$HqWuK6/Ng6sg9gQzbLrgb.Tl.ZHfXLhvt/SgVyWhQqgqcZ7ZuUtye",
    "$2a$10$k1wbIrmNyFAPwPVPSVa/zecw2BCEnBwVS2GbrmgzxFUOqW9dk4TCW",
    "$2a$12$k42ZFHFWqBp3vWli.nIn8uYyIkbvYRvodzbfbK18SSsY.CsIQPlxO",
    "$2b$10$k1wbIrmNyFAPwPVPSVa/zecw2BCEnBwVS2GbrmgzxFUOqW9dk4TCW",
    "$2y$10$k1wbIrmNyFAPwPVPSVa/zecw2BCEnBwVS2GbrmgzxFUOqW9dk4TCW",
];

/** Sixty characters that are not a hash, one per rule being pinned. */
const MALFORMED = [
    // No dollar at all: no third field, so the rounds print as NaN -- and the
    // salt occurs again at twenty-nine, so the password hash is empty.
    sixty(""),
    // A third field that is empty, which parseInt also calls NaN.
    sixty("$2a$" + "$"),
    // A third field with no digits, and one with digits only after text.
    sixty("$2a$abc$"),
    sixty("$2a$abc10$"),
    // Digits followed by text, which parseInt takes the prefix of.
    sixty("$2a$10rounds$"),
    // Leading whitespace and a sign, all three of which parseInt honours.
    sixty("$2a$ 12$"),
    sixty("$2a$-5$"),
    sixty("$2a$+5$"),
    // A leading zero, which at radix ten is just a zero.
    sixty("$2a$007$"),
    // Thirty digits: past what a double counts by ones, so the answer is
    // rounded and prints in exponential notation.
    sixty("$2a$" + "9".repeat(30) + "$"),
    // Twenty digits, which is under the exponential threshold and over the
    // exactly-representable one -- so it prints in full, with the digits the
    // rounding produced rather than the ones that were written.
    sixty("$2a$" + "1".repeat(20) + "$"),
    // The salt occurring again at thirty rather than at twenty-nine, so the
    // password hash is one character: a proper piece of the rest, neither all
    // of it nor none of it.
    "a".repeat(29) + "b" + "a".repeat(29) + "b",
];

export async function add({addCase}) {
    let index = 0;
    for (const hash of [...HASHES, ...MALFORMED]) {
        if (hash.length !== LENGTH) {
            throw new Error(`case ${index} is ${hash.length} characters, not ${LENGTH}`);
        }
        addCase(`bcrypt_parse_${index}`, hash, [{op: "Bcrypt parse", args: []}]);
        index += 1;
    }
}
