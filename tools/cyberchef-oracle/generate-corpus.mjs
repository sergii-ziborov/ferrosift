// Automatic differential corpus generator.
//
// Deterministically samples inputs for every CyberChef-aliased FerroSift
// operation, bakes each recipe through the pinned checkout, and records the
// exact output bytes at every recipe prefix. `tests/corpus.rs` replays the
// result and asserts byte-for-byte equality and stopping positions.
//
// Determinism: a seeded xorshift PRNG, no clock, no Math.random. Decoder and
// decompressor inputs derive from the paired reference encoder at generation
// time, so every decode case is canonical by construction.
//
// The samplers live in ./corpus/builder.mjs and the cases in one module per
// operation family. Family order below is part of the fixture: it fixes both
// the PRNG draw order and the order cases are written in.
import {mkdir, writeFile} from "node:fs/promises";
import path from "node:path";

import {
    COMMIT,
    VERSION,
    bakeHex,
    bakeString as bakeStringWith,
    fixtureDir,
    loadChef,
    makeInput,
} from "./reference.mjs";
import {createBuilder} from "./corpus/builder.mjs";
import * as bitwise from "./corpus/bitwise.mjs";
import * as casing from "./corpus/casing.mjs";
import * as shaping from "./corpus/shaping.mjs";
import * as unicodeEscape from "./corpus/unicode-escape.mjs";
import * as brute from "./corpus/brute.mjs";
import * as misc from "./corpus/misc.mjs";
import * as substitute from "./corpus/substitute.mjs";
import * as checksum from "./corpus/checksum.mjs";
import * as classical from "./corpus/classical.mjs";
import * as compress from "./corpus/compress.mjs";
import * as crypto from "./corpus/crypto.mjs";
import * as digest from "./corpus/digest.mjs";
import * as encoding from "./corpus/encoding.mjs";
import * as extract from "./corpus/extract.mjs";
import * as legacyDigest from "./corpus/legacy-digest.mjs";
import * as sets from "./corpus/sets.mjs";
import * as shape from "./corpus/shape.mjs";
import * as text from "./corpus/text.mjs";

const chef = await loadChef();
const output = path.join(fixtureDir, "corpus.json");

const builder = createBuilder({
    bakeString: (input, recipe) => bakeStringWith(chef, input, recipe),
    seed: 0x5f37_1d10,
});

for (const family of [encoding, text, digest, crypto, compress, extract, shape, bitwise, classical, checksum, sets, legacyDigest, casing, shaping, unicodeEscape, brute, misc, substitute]) {
    await family.add(builder);
}

const {cases} = builder;
let failures = 0;
for (const testCase of cases) {
    testCase.outputs_hex = [];
    for (let length = 1; length <= testCase.recipe.length; length += 1) {
        try {
            testCase.outputs_hex.push(
                await bakeHex(chef, makeInput(testCase.input), testCase.recipe.slice(0, length)),
            );
        } catch (error) {
            failures += 1;
            process.stderr.write(
                `bake failed: ${testCase.name} prefix ${length}: ${error?.message ?? error}\n`,
            );
            break;
        }
    }
    testCase.stopped_after = testCase.outputs_hex.length;
}

const complete = cases.filter(testCase => testCase.stopped_after === testCase.recipe.length);

const suite = {
    reference: {name: "CyberChef", version: VERSION, commit: COMMIT},
    cases: complete,
};

await mkdir(path.dirname(output), {recursive: true});
await writeFile(output, `${JSON.stringify(suite, null, 1)}\n`, "utf8");
process.stdout.write(
    `wrote ${complete.length} corpus cases (${failures} bake failures dropped) to ${output}\n`,
);
