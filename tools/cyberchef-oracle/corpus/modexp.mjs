// Modular exponentiation, which the reference introduced in 11.4.
//
// The first family here whose operation 11.3 has never heard of. Under the
// baseline profile every case below fails to bake and is dropped; under 11.4
// they bake, so they arrive in the overlay's `added` list. That asymmetry is
// the evidence for the alias: a name only the newer reference answers to.
//
// The interesting cases are the ones outside the textbook. The reference's
// square-and-multiply loop is guarded by `exponent > 0n`, so a negative
// exponent returns one rather than throwing; `%` keeps the sign of the
// dividend, so a negative base or modulus produces a negative result; and a
// modulus of one with a zero exponent returns one rather than zero, because
// the reduction that would have taken it to zero never runs. None of those are
// what a textbook says modular exponentiation does, and all of them are what a
// recipe written against the reference produces.

// A 617-digit value, comfortably past every fixed width. Shared shape with the
// Extended GCD family: the point of arbitrary precision is the large cases.
const HUGE = "2519590847565789349402718324004839857142928212620403202777713783604366202070"
    + "7595556264018525880784406918290641249515082189298559149176184502808489120072"
    + "8449926873928072877767359714183472702618963750149718246911650776133798590957"
    + "0009733045974880842840179742910064245869181719511874612151517265463228221686"
    + "9987549182422433637259085141865462043576798423387184774447920739934236584823"
    + "8242811981638150106748104516603773060562016196762561338441436038339044149526"
    + "3443219011465754445417842402092461651572335077870774981712577246796292638635"
    + "6373289912154831438167899885040445364023527381951378636564391212010397122822"
    + "120720357";

// [base, modulus, exponent] — the reference's own argument order, which puts
// the modulus in the middle. A positional recipe carries them this way round.
const TRIPLES = [
    // Textbook.
    ["4", "497", "13"],
    ["2", "1000000007", "1000"],
    ["3", "11", "7"],
    // Zero and one, where the loop's guard decides the answer.
    ["0", "7", "0"],
    ["0", "7", "5"],
    ["7", "3", "0"],
    ["5", "1", "0"],
    ["5", "1", "3"],
    ["1", "1", "1"],
    // A negative exponent never enters the loop.
    ["3", "7", "-2"],
    ["3", "7", "-1"],
    // A negative base or modulus keeps the sign of the dividend through `%`.
    ["-3", "7", "3"],
    ["-3", "7", "2"],
    ["3", "-7", "3"],
    ["-3", "-7", "3"],
    // Signs and leading zeros the decimal pattern accepts.
    ["+4", "497", "+13"],
    ["004", "497", "013"],
    // Hex, which the reference accepts only unsigned.
    ["0xff", "0x101", "0x10"],
    ["0xFF", "257", "16"],
    // Arguments the operation trims before reading.
    ["  4  ", " 497 ", "\t13\n"],
    // U+FEFF is whitespace to `String.prototype.trim` and not to Rust's
    // `str::trim`. Pinned because the two disagree and the reference's set is
    // the one that decides whether this parses at all.
    ["﻿4", "497", "13﻿"],
    // The case the operation exists for: RSA-shaped exponentiation.
    [HUGE, HUGE, "65537"],
    ["2", HUGE, HUGE],
    ["65537", HUGE, "3"],
];

export async function add({addCase}) {
    for (const [index, [base, modulus, exponent]] of TRIPLES.entries()) {
        addCase(`modexp_args_${index}`, "", [
            {op: "Modular Exponentiation", args: [base, modulus, exponent]},
        ]);
    }

    // Exactly one of base and exponent may come from the input. Both
    // directions are pinned because implementing them the wrong way round
    // produces a plausible number rather than an error.
    for (const [index, [base, modulus, exponent]] of TRIPLES.entries()) {
        addCase(`modexp_input_base_${index}`, base, [
            {op: "Modular Exponentiation", args: ["", modulus, exponent]},
        ]);
        addCase(`modexp_input_exponent_${index}`, exponent, [
            {op: "Modular Exponentiation", args: [base, modulus, ""]},
        ]);
    }

    // With both boxes filled the input is ignored rather than consulted.
    addCase("modexp_input_ignored", "999999", [
        {op: "Modular Exponentiation", args: ["4", "497", "13"]},
    ]);

    // Whitespace-only counts as empty, so these fall back to the input.
    addCase("modexp_whitespace_base", "4", [
        {op: "Modular Exponentiation", args: ["   ", "497", "13"]},
    ]);
    addCase("modexp_whitespace_exponent", "13", [
        {op: "Modular Exponentiation", args: ["4", "497", " \t "]},
    ]);

    // Feeding the result back in: the second step takes its base from the
    // first step's output, which is where a port that reads the input at the
    // wrong moment comes apart.
    addCase("modexp_chained", "", [
        {op: "Modular Exponentiation", args: ["4", "497", "13"]},
        {op: "Modular Exponentiation", args: ["", "1009", "7"]},
    ]);
}
