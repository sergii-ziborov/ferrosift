// Curated CyberChef differential suite.
//
// Hand-picked recipes that exercise representative and quirk-prone paths.
// Each case records the reference output at every recipe prefix, so a
// divergence is reported at the step that caused it.
//
// The cases live in ./suite, one module per operation family; this file only
// bakes them against the pinned reference and writes the fixture.
import {mkdir, writeFile} from "node:fs/promises";
import path from "node:path";

import {COMMIT, VERSION, bakeEveryPrefix, fixtureDir, loadChef} from "./reference.mjs";
import {curatedCases, unsupportedCase} from "./suite/index.mjs";

const chef = await loadChef();
const output = path.join(fixtureDir, "differential.json");

for (const testCase of curatedCases) {
    try {
        testCase.outputs_hex = await bakeEveryPrefix(chef, testCase);
    } catch (error) {
        throw new Error(`${testCase.name} failed to bake`, {cause: error});
    }
    testCase.stopped_after = testCase.outputs_hex.length;
}

const suite = {
    reference: {name: "CyberChef", version: VERSION, commit: COMMIT},
    cases: curatedCases,
    unsupported: unsupportedCase,
};

await mkdir(path.dirname(output), {recursive: true});
await writeFile(output, `${JSON.stringify(suite, null, 2)}\n`, "utf8");
process.stdout.write(`wrote ${curatedCases.length} cases to ${output}\n`);
