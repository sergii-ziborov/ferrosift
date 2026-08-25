// Arbitrary-precision integer arithmetic.
//
// The point of arbitrary precision is the large cases, so those are pinned
// rather than only the textbook ones. A 2048-bit modulus is the input Modular
// Inverse exists for, and a fixed-width port would have been wrong there and
// right everywhere else — which is exactly the divergence a corpus of small
// numbers would miss.

// A 617-digit semiprime-ish value, well past what 64 or 128 bits hold.
const HUGE = "2519590847565789349402718324004839857142928212620403202777713783604366202070"
    + "7595556264018525880784406918290641249515082189298559149176184502808489120072"
    + "8449926873928072877767359714183472702618963750149718246911650776133798590957"
    + "0009733045974880842840179742910064245869181719511874612151517265463228221686"
    + "9987549182422433637259085141865462043576798423387184774447920739934236584823"
    + "8242811981638150106748104516603773060562016196762561338441436038339044149526"
    + "3443219011465754445417842402092461651572335077870774981712577246796292638635"
    + "6373289912154831438167899885040445364023527381951378636564391212010397122822"
    + "120720357";

const GCD_PAIRS = [
    ["12", "18"],
    ["270", "192"],
    ["17", "5"],
    ["0", "5"],
    ["5", "0"],
    ["0", "0"],
    ["1", "1"],
    // Negative operands, where the sign of the gcd and of the coefficients is
    // the part a port gets wrong.
    ["-12", "18"],
    ["12", "-18"],
    ["-12", "-18"],
    // Hex input, which the reference accepts only without a sign.
    ["0xff", "0x10"],
    ["0xFF", "16"],
    // Past every fixed width.
    [HUGE, "65537"],
    [HUGE, "3"],
];

const INVERSE_PAIRS = [
    ["3", "11"],
    ["10", "17"],
    ["1", "2"],
    ["7", "26"],
    // Not coprime, and a zero value: the reference throws for both, so they
    // cannot be pinned as output. tests/conformance_bigint.rs holds them.
    // Negative value, normalised into the modulus first.
    ["-3", "11"],
    ["-1", "7"],
    ["0x10", "0xff"],
    // The case this operation exists for.
    ["65537", HUGE],
];

export async function add({addCase}) {
    for (const [index, [a, b]] of GCD_PAIRS.entries()) {
        // Both operands as arguments.
        addCase(`egcd_args_${index}`, "", [{op: "Extended GCD", args: [a, b]}]);
        // First from the input, second as an argument — the reference's own
        // fallback, which is easy to implement backwards.
        addCase(`egcd_input_a_${index}`, a, [{op: "Extended GCD", args: ["", b]}]);
        // Second from the input.
        addCase(`egcd_input_b_${index}`, b, [{op: "Extended GCD", args: [a, ""]}]);
    }

    for (const [index, [a, m]] of INVERSE_PAIRS.entries()) {
        addCase(`modinv_args_${index}`, "", [{op: "Modular Inverse", args: [a, m]}]);
        addCase(`modinv_input_a_${index}`, a, [{op: "Modular Inverse", args: ["", m]}]);
        addCase(`modinv_input_m_${index}`, m, [{op: "Modular Inverse", args: [a, ""]}]);
    }

    // Whitespace-only arguments count as missing, so these take both from the
    // input and refuse — pinned so the trim is not quietly dropped.
    addCase("egcd_whitespace_args", "12", [{op: "Extended GCD", args: ["   ", "18"]}]);
    addCase("modinv_whitespace_args", "3", [{op: "Modular Inverse", args: ["  ", "11"]}]);
}
