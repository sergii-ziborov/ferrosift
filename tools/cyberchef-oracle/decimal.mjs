// Pins how the reference renders an arbitrary-precision number.
//
// FerroSift carries one as a canonical sign / digits / exponent rather than by
// depending on `bignumber.js`, so the rendering has to be reproduced. This
// takes the truth from the library itself instead of from its documentation:
// every pair below is what the real `BigNumber` produced, not what a reading
// of the spec suggested it would.
//
// The dish converts with `toFixed()` and no argument, which is *not*
// `toString()`. `toFixed()` never uses exponential notation, whatever the
// exponent -- so a value that `toString` would write as `1e+25` is written out
// in full here. Reproducing `toString` instead would be wrong in exactly the
// cases that are hardest to notice: the very large and the very small.

import {writeFileSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

import {selectedProfile, verifyCheckout} from "./reference.mjs";

const profile = selectedProfile();
const checkout = verifyCheckout(profile);
const {default: BigNumber} = await import(
    new URL(`file://${path.join(checkout, "node_modules/bignumber.js/dist/bignumber.mjs")}`).href
);

const INPUTS = [
    // Whole numbers, including the boundaries a float would lose.
    "0", "-0", "1", "-1", "42", "9007199254740993", "-9007199254740993",
    // Far past any fixed width.
    "123456789012345678901234567890",
    "-123456789012345678901234567890",
    // Fractions, including ones with no exact binary form.
    "0.1", "0.5", "-0.25", "3.14159265358979323846264338327950288",
    "0.000000000000000000001",
    // Exponential input, which `toFixed` writes out in full.
    "1e+25", "1e-25", "1.5e+30", "-2.5e-30", "1e21", "1e20",
    // Trailing and leading zeroes, which normalisation must not invent or lose.
    "1.000", "0.0100", "000123", "-000.500",
    // The specials.
    "NaN", "Infinity", "-Infinity",
    // Rejected input becomes NaN rather than an error.
    "", "abc", "1.2.3", "--5",
];

// The constructor *throws* on input it cannot read -- it does not return NaN,
// which is what the documentation's talk of NaN values suggests. The dish
// catches and substitutes `new BigNumber(NaN)`, so that is the behaviour to
// record: what a recipe observes, not what the library does on its own.
const cases = INPUTS.map(input => {
    let value;
    let threw = false;
    try {
        value = new BigNumber(input);
    } catch {
        threw = true;
        value = new BigNumber(NaN);
    }
    return {input, fixed: value.toFixed(), nan: value.isNaN(), rejected: threw};
});

const output = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../../crates/ferrosift-model/tests/fixtures/decimal.json",
);
writeFileSync(
    output,
    `${JSON.stringify({library: "bignumber.js", method: "toFixed()", cases}, null, 1)}\n`,
    "utf8",
);
process.stdout.write(`wrote ${cases.length} decimal cases to ${output}\n`);
