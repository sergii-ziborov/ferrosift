// Pins the arithmetic of the reference's arbitrary-precision library.
//
// The rendering was pinned first, and rendering is the easy half: it has one
// answer per value. Arithmetic has a configuration, and the configuration is
// where a port goes wrong quietly. This records what the library actually does
// with its own defaults rather than what its documentation says they are --
// the same reading that already found the exponent range to be ten million
// where the documentation says a billion.
//
// The three settings that decide everything here:
//
//   DECIMAL_PLACES 20  -- how far an inexact division or root is carried
//   ROUNDING_MODE   4  -- half away from zero, at that last place
//   MODULO_MODE     1  -- truncated, so a remainder takes the dividend's sign
//
// Addition, subtraction and multiplication are exact and ignore all three.
// Division and square root are not, and that asymmetry is the thing a port
// most easily gets wrong: rounding a sum, or failing to round a quotient.

import {writeFileSync, mkdirSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

import {selectedProfile, verifyCheckout} from "./reference.mjs";

const checkout = verifyCheckout(selectedProfile());
const fromCheckout = async relative =>
    import(new URL(`file://${path.join(checkout, relative)}`).href);

const {default: BigNumber} = await fromCheckout(
    "node_modules/bignumber.js/dist/bignumber.mjs",
);

// The folds come from the reference's own arithmetic library rather than from
// a transcription of it here. Its `sum` reduces without a seed, its `mean`
// divides by a plain count, and its `median` sorts with a comparator that
// returns a BigNumber for the runtime to coerce -- three details that a
// transcription would be free to get wrong in the same way the port does.
const arithmetic = await fromCheckout("src/core/lib/Arithmetic.mjs");

/// Pairs chosen so each one asks a different question.
const PAIRS = [
    // Exactness that a float would lose.
    ["0.1", "0.2"],
    ["0.3", "0.1"],
    ["9007199254740993", "1"],
    ["123456789012345678901234567890", "987654321"],
    // Signs, in every combination.
    ["7", "3"], ["-7", "3"], ["7", "-3"], ["-7", "-3"],
    // Fractions, where a remainder is not whole.
    ["7.5", "2"], ["-7.5", "2"],
    // Division that does not terminate, which is where rounding shows.
    ["1", "3"], ["2", "3"], ["-2", "3"], ["1", "7"], ["10", "3"],
    // Division that terminates well inside the limit.
    ["10", "4"], ["1", "8"],
    // A quotient whose twentieth place is exactly a half, which is what the
    // rounding mode is for.
    ["1", "2"], ["3", "8"],
    // Zero on either side.
    ["0", "5"], ["5", "0"], ["0", "0"], ["-5", "0"],
    // The specials, which propagate rather than fail.
    ["NaN", "1"], ["Infinity", "1"], ["1", "Infinity"],
    ["Infinity", "Infinity"], ["-Infinity", "2"],
    // Very small and very large together.
    ["1e-20", "1e20"], ["1e20", "1e-20"],
];

/// Values for the one-argument operations.
const SINGLES = [
    "0", "1", "2", "4", "9", "100", "0.25", "2.25",
    "-1", "-0", "1e-40", "1e40",
    "9007199254740993",
    "NaN", "Infinity", "-Infinity",
    // Odd exponents. A root has to apply an odd power of ten inside itself,
    // and an implementation that quietly evened the power out -- by moving the
    // exponent and forgetting to move the coefficient with it -- would answer
    // every case above correctly, because every one of them is even once
    // normalised. These are the cases that tell the difference.
    "2000", "0.002", "0.0002", "2e-7", "3e9", "1.5e-9",
    // Small enough that the root falls near the last place kept, which is
    // where the decision to answer zero has to be made carefully: the root
    // halves the scale, so a value that looks negligible may not be.
    "1e-41", "9e-41", "1e-42", "9e-43", "1e-44", "1e-50",
    "123456789012345678901234567890e-45",
    // A root that is not exact, where the twentieth place is decided by what
    // comes after it rather than by the digit itself.
    "3", "5", "7", "10", "0.1", "1e-21",
];

/// Lists for the fold operations, which the arithmetic operations build on.
const LISTS = [
    ["1", "2", "3", "4"],
    ["1", "2", "3"],
    ["0.1", "0.2", "0.3"],
    ["-5", "5", "-2.5"],
    ["1e20", "1", "-1e20"],
    ["7"],
    ["2", "3"],
    ["9007199254740993", "9007199254740993"],
];

const render = value => {
    try {
        return value.toFixed();
    } catch {
        return "<unrenderable>";
    }
};

const binary = [];
for (const [left, right] of PAIRS) {
    const a = new BigNumber(left);
    const b = new BigNumber(right);
    binary.push({
        left,
        right,
        plus: render(a.plus(b)),
        minus: render(a.minus(b)),
        times: render(a.times(b)),
        div: render(a.div(b)),
        mod: render(a.mod(b)),
    });
}

const unary = SINGLES.map(input => {
    const value = new BigNumber(input);
    return {
        input,
        sqrt: render(value.sqrt()),
        negated: render(value.negated()),
        absolute: render(value.abs()),
    };
});

// `median` sorts in place, so each fold gets its own copy -- otherwise the
// order recorded for one answer would depend on which ran before it.
const lists = LISTS.map(values => ({
    values,
    sum: render(arithmetic.sum(values.map(value => new BigNumber(value)))),
    mean: render(arithmetic.mean(values.map(value => new BigNumber(value)))),
    median: render(arithmetic.median(values.map(value => new BigNumber(value)))),
}));

const outputDir = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../../crates/ferrosift-operations/tests/fixtures",
);
mkdirSync(outputDir, {recursive: true});
const output = path.join(outputDir, "bignumber.json");
const config = BigNumber.config();
writeFileSync(
    output,
    `${JSON.stringify(
        {
            library: "bignumber.js",
            settings: {
                decimal_places: config.DECIMAL_PLACES,
                rounding_mode: config.ROUNDING_MODE,
                modulo_mode: config.MODULO_MODE,
            },
            binary,
            unary,
            lists,
        },
        null,
        1,
    )}\n`,
    "utf8",
);
process.stdout.write(
    `wrote ${binary.length} binary, ${unary.length} unary and ${lists.length} list cases to ${output}\n`,
);
