// Arithmetic over a delimited list of decimals.
//
// The reference builds these on `bignumber.js` rather than on JavaScript's own
// numbers, and the cases below are chosen so that a port built on floats would
// fail rather than merely be imprecise: tenths that a float cannot hold, whole
// numbers past 2^53, and a quotient that does not terminate.
//
// Three things beyond the arithmetic are pinned here because each is a rule a
// port would have to invent otherwise, and could invent differently:
//
//   - which tokens count as numbers, since the reference silently drops the
//     rest and reads `0x0a` as ten;
//   - what an empty list answers, since the reference's fold has no seed and
//     the operation substitutes not-a-number;
//   - how MOD writes its answers, since it joins them itself and therefore
//     gets `toString` where every other operation here gets `toFixed`.

// The examples from the reference's own descriptions. If a port disagreed with
// its documentation the disagreement should be visible, so these come first.
const DOCUMENTED = [
    ["Sum", "0x0a 8 .5"],
    ["Subtract", "0x0a 8 .5"],
    ["Multiply", "0x0a 8 .5"],
    ["Divide", "0x0a 8 .5"],
    ["Mean", "0x0a 8 .5 .5"],
    ["Median", "0x0a 8 1 .5"],
    ["Standard Deviation", "0x0a 8 .5"],
];

const AGGREGATES = [
    "Sum", "Subtract", "Multiply", "Divide", "Mean", "Median", "Standard Deviation",
];

const LISTS = [
    // Exactness a float would lose. `0.1 0.2` is the case the whole dependency
    // exists for.
    "0.1 0.2",
    "0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9 1.0",
    // Past 2^53, where a float stops counting by ones.
    "9007199254740993 1",
    "9007199254740993 9007199254740993",
    "123456789012345678901234567890 987654321",
    // Division that does not terminate, which is where the rounding shows.
    "1 3",
    "2 3 7",
    "10 3",
    // Signs, in every arrangement.
    "-7 3", "7 -3", "-7 -3", "-1 -2 -3",
    // Zero, including a division by it.
    "0 5", "5 0", "0 0",
    // One item, which a fold with a seed would get wrong for subtraction and
    // division, and none at all, which answers not-a-number.
    "42", "-42", "",
    // Tokens that are not numbers, which are dropped rather than reported.
    "1 apples 2 3", "apples pears", "1 NaN 2", "1 Infinity 2",
    // The prefixed bases the single-argument constructor reads.
    "0x0a 0b101 0o17",
    // An even count and an odd one, which are the two halves of a median.
    "5 1 4 2", "5 1 4 2 3",
    // Already ordered, and ordered backwards: a median that sorted by text
    // rather than by value would answer differently for one of these.
    "1 2 10", "10 2 1",
    // Only whitespace, and a trailing delimiter -- both give empty tokens,
    // which are not numbers.
    "   ", "1 2 3 ",
    // Very small and very large together, where the exponent does the work.
    "1e-20 1e20", "1e20 1e-20",
];

const DELIMITERS = [
    ["Line feed", "\n"],
    ["Space", " "],
    ["Comma", ","],
    ["Semi-colon", ";"],
    ["Colon", ":"],
    ["CRLF", "\r\n"],
];

// Moduli and the lists to reduce. The small values matter most: a remainder
// below a ten-millionth is written exponentially by `toString`, which is the
// half of the rendering that `toFixed` never reaches.
const MOD_CASES = [
    [3, "15 4 7"],
    [2, "1 2 3 4 5"],
    [5, "-1 -6 -11"],
    [7, "0.5 1.5 2.5"],
    [3, "0.00000001 0.0000001 0.000001"],
    [3, "1e-8 2e-9"],
    [1000000, "9007199254740993 123456789012345678901234567890"],
    [2, ""],
    [2, "apples"],
    [4, "0x0a 0b101"],
    [-3, "15 4 7"],
    [1, "5 5.5 -5.5"],
];

export async function add({addCase}) {
    for (const [index, [op, input]] of DOCUMENTED.entries()) {
        addCase(`arith_documented_${index}`, input, [{op, args: ["Space"]}]);
    }

    for (const op of AGGREGATES) {
        const slug = op.toLowerCase().replace(/ /gu, "_");
        for (const [index, list] of LISTS.entries()) {
            addCase(`arith_${slug}_${index}`, list, [{op, args: ["Space"]}]);
        }
    }

    // Every delimiter, on one operation rather than on all seven: the reading
    // of the input is shared code there and here, so testing it once per
    // delimiter says as much as testing it seven times would.
    for (const [index, [name, separator]] of DELIMITERS.entries()) {
        addCase(`arith_delim_${index}`, ["0x0a", "8", ".5"].join(separator), [
            {op: "Sum", args: [name]},
        ]);
        addCase(`arith_delim_mod_${index}`, ["15", "4", "7"].join(separator), [
            {op: "MOD", args: [3, name]},
        ]);
    }

    for (const [index, [modulus, list]] of MOD_CASES.entries()) {
        addCase(`arith_mod_${index}`, list, [{op: "MOD", args: [modulus, "Space"]}]);
    }

    // A number carried into the next step: the dish holds a BigNumber, and
    // what the next operation receives is that number rendered with `toFixed`.
    // Pinning the pair catches a port that agreed about the answer and
    // disagreed about how it is handed on.
    addCase("arith_chain_sum_to_hex", "1e-20 1e20", [
        {op: "Sum", args: ["Space"]},
        {op: "To Hex", args: ["Space", 0]},
    ]);
    addCase("arith_chain_divide_to_upper", "1 3", [
        {op: "Divide", args: ["Space"]},
        {op: "To Upper case", args: ["All"]},
    ]);
    addCase("arith_chain_mean_to_sum", "1 2 3 4", [
        {op: "Mean", args: ["Space"]},
        {op: "Sum", args: ["Space"]},
    ]);
}
