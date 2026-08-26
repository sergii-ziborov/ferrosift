// Pins how the reference renders an arbitrary-precision number.
//
// FerroSift carries one as a canonical sign / digits / exponent rather than by
// depending on `bignumber.js`, so the rendering has to be reproduced. This
// takes the truth from the library itself instead of from its documentation:
// every pair below is what the real `BigNumber` produced, not what a reading
// of the spec suggested it would.
//
// Both renderings are recorded, because the reference uses both. The dish
// converts with `toFixed()` and no argument, which never uses exponential
// notation whatever the exponent -- so a value that `toString` would write as
// `1e+25` is written out in full. But an operation that joins numbers into a
// string of its own gets `toString`, which does use it: MOD is one, and a port
// carrying only `toFixed` would be right about a remainder of `2.5` and wrong
// about a remainder of `1e-8`.
//
// The thresholds at which `toString` switches are recorded too. The library's
// documentation gives the positive one as twenty; the value below is what the
// library actually reports, and the cases either side of it show which is
// right.

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

    // Whitespace, which is the likeliest place for a port to drift. Rust's
    // `trim` removes every character with the Unicode White_Space property;
    // whatever the reference accepts is a different set, and the difference is
    // invisible until a value carries one of the characters they disagree on.
    " 1 ", "\t1\t", "\n1\n", "\r\n1\r\n",
    " 1 ",   // no-break space
    "﻿1",         // byte-order mark, which Rust does not call whitespace
    "1",         // next line
    " 1",         // line separator
    "　1",         // ideographic space
    "1", "1",
    " ",               // whitespace and nothing else

    // The exponent range. The library clamps beyond its own limit rather than
    // carrying the exponent, so a port that kept it would render a number the
    // reference calls infinite -- or spend the memory trying.
    "1e999999999", "1e1000000000", "1e-1000000000",
    "1e9007199254740992", "1e-9007199254740992",

    // Signs on the exponent, and an exponent with no digits.
    "1e+", "1e", "1e+0", "1E5", "1e05",

    // A lone point, and a point with digits on only one side.
    ".", ".5", "5.", "-.5", "+.5",

    // Other bases, which the single-argument constructor may or may not read.
    "0x1f", "0b101", "0o17",

    // Either side of the thresholds where `toString` turns exponential, which
    // `toFixed` never does. The positive one is the interesting half: the
    // documentation says twenty, so `1e20` and `1e21` are the pair that
    // settles it.
    "1e19", "1e21", "1e22", "1.5e20", "1.5e21", "-1e21",
    "1e-6", "1e-7", "1e-8", "1.5e-7", "-1e-8",
    "1234567890123456789012345",
    "0.0000001", "0.00000001",
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
    return {
        input,
        fixed: value.toFixed(),
        // `String(value)` rather than `value.toString()`, because that is what
        // `Array.prototype.join` actually calls.
        written: String(value),
        nan: value.isNaN(),
        rejected: threw,
    };
});

const output = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../../crates/ferrosift-model/tests/fixtures/decimal.json",
);
writeFileSync(
    output,
    `${JSON.stringify(
        {
            library: "bignumber.js",
            methods: ["toFixed()", "toString()"],
            exponential_at: BigNumber.config().EXPONENTIAL_AT,
            cases,
        },
        null,
        1,
    )}\n`,
    "utf8",
);
process.stdout.write(`wrote ${cases.length} decimal cases to ${output}\n`);
