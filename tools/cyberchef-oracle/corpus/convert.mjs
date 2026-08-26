// Converting a quantity between units.
//
// The five operations share one body, so what the corpus is really checking is
// the hundred and eighty-nine factors behind them. Those are generated from the
// checkout rather than typed, which removes the transcription risk but not the
// risk that the generator read the wrong thing -- so every unit in every table
// appears here at least once, converted against its group's base.
//
// A sampled sweep would have been cheaper and would have left most of the
// table unchecked. One mistyped or misread factor is a wrong answer for one
// unit out of thirty-five, in an operation whose other answers all look right,
// which is exactly the failure a sample misses.

import path from "node:path";
import {readFile} from "node:fs/promises";

import {selectedProfile, verifyCheckout} from "../reference.mjs";

const checkout = verifyCheckout(selectedProfile());

/** The five operations and the table each consults. */
const GROUPS = [
    {op: "Convert area", module: "ConvertArea", table: "AREA_FACTOR", base: "Square metre (sq m)"},
    {op: "Convert data units", module: "ConvertDataUnits", table: "DATA_FACTOR", base: "Bytes (B)"},
    {op: "Convert distance", module: "ConvertDistance", table: "DISTANCE_FACTOR", base: "Metres (m)"},
    {op: "Convert mass", module: "ConvertMass", table: "MASS_FACTOR", base: "Kilogram (kg)"},
    {op: "Convert speed", module: "ConvertSpeed", table: "SPEED_FACTOR", base: "Metres per second (m/s)"},
];

/** Evaluates one `const NAME = { ... };` object out of a module's source. */
function extract(text, name) {
    const at = text.indexOf(`const ${name}`);
    if (at < 0) throw new Error(`no ${name} in source`);
    const open = text.indexOf("{", at);
    let depth = 0;
    let end = open;
    for (; end < text.length; end += 1) {
        if (text[end] === "{") depth += 1;
        else if (text[end] === "}") {
            depth -= 1;
            if (depth === 0) break;
        }
    }
    // eslint-disable-next-line no-new-func
    return Function(`"use strict"; return (${text.slice(open, end + 1)});`)();
}

/// Values chosen so the answers exercise more than one shape.
const QUANTITIES = ["1", "0", "-1", "2.5", "1000000", "0.000001", "9007199254740993"];

export async function add({addCase}) {
    for (const group of GROUPS) {
        const file = path.join(checkout, "src/core/operations", `${group.module}.mjs`);
        const units = Object.keys(extract(await readFile(file, "utf8"), group.table));

        // Every unit, out of the base and back into it. Two cases per unit
        // rather than one: the factor is used as a multiplier in one direction
        // and as a divisor in the other, and only the divisor rounds.
        for (const [index, unit] of units.entries()) {
            addCase(`convert_${group.module}_out_${index}`, "1", [
                {op: group.op, args: [group.base, unit]},
            ]);
            addCase(`convert_${group.module}_in_${index}`, "1", [
                {op: group.op, args: [unit, group.base]},
            ]);
        }

        // A handful of quantities through one awkward pair, where the two
        // factors are far apart and the division does not terminate.
        const first = units[0];
        const last = units[units.length - 1];
        for (const [index, quantity] of QUANTITIES.entries()) {
            addCase(`convert_${group.module}_q${index}`, quantity, [
                {op: group.op, args: [first, last]},
            ]);
            addCase(`convert_${group.module}_r${index}`, quantity, [
                {op: group.op, args: [last, first]},
            ]);
        }

        // Through the pipeline: a number arrives from the dish and leaves as
        // one, which is what makes these usable in a recipe at all.
        addCase(`convert_${group.module}_chained`, "1000", [
            {op: group.op, args: [group.base, units[1]]},
            {op: group.op, args: [units[1], group.base]},
        ]);
    }
}
