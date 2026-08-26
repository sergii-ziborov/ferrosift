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
    bakeOutput,
    bakeString as bakeStringWith,
    fixtureDirFor,
    loadChef,
    makeInput,
    selectedProfile,
} from "./reference.mjs";
import {createBuilder} from "./corpus/builder.mjs";
import * as bitwise from "./corpus/bitwise.mjs";
import * as casing from "./corpus/casing.mjs";
import * as shaping from "./corpus/shaping.mjs";
import * as unicodeEscape from "./corpus/unicode-escape.mjs";
import * as brute from "./corpus/brute.mjs";
import * as misc from "./corpus/misc.mjs";
import * as crosskind from "./corpus/crosskind.mjs";
import * as mail from "./corpus/mail.mjs";
import * as numeric from "./corpus/numeric.mjs";
import * as substitute from "./corpus/substitute.mjs";
import * as netfmt from "./corpus/netfmt.mjs";
import * as markup from "./corpus/markup.mjs";
import * as varint from "./corpus/varint.mjs";
import * as braille from "./corpus/braille.mjs";
import * as annotate from "./corpus/annotate.mjs";
import * as bigint from "./corpus/bigint.mjs";
import * as checksum from "./corpus/checksum.mjs";
import * as classical from "./corpus/classical.mjs";
import * as compress from "./corpus/compress.mjs";
import * as crypto from "./corpus/crypto.mjs";
import * as digest from "./corpus/digest.mjs";
import * as encoding from "./corpus/encoding.mjs";
import * as extract from "./corpus/extract.mjs";
import * as framing from "./corpus/framing.mjs";
import * as legacyDigest from "./corpus/legacy-digest.mjs";
import * as sets from "./corpus/sets.mjs";
import * as shape from "./corpus/shape.mjs";
import * as text from "./corpus/text.mjs";
import * as sponge from "./corpus/sponge.mjs";
import * as snort from "./corpus/snort.mjs";
import * as bacon from "./corpus/bacon.mjs";
import * as legacyHash from "./corpus/legacy.mjs";
import * as bifid from "./corpus/bifid.mjs";
import * as caseregex from "./corpus/caseregex.mjs";
import * as unixperms from "./corpus/unixperms.mjs";
import * as rc4drop from "./corpus/rc4drop.mjs";
import * as punycode from "./corpus/punycode.mjs";
import * as bech32 from "./corpus/bech32.mjs";
import * as ls47 from "./corpus/ls47.mjs";
import * as stats from "./corpus/stats.mjs";
import * as offsetcheck from "./corpus/offsetcheck.mjs";

const profile = selectedProfile();
const chef = await loadChef(profile);
const output = path.join(fixtureDirFor(profile), "corpus.json");

const builder = createBuilder({
    bakeString: (input, recipe) => bakeStringWith(chef, input, recipe),
    seed: 0x5f37_1d10,
});

// Order is part of the fixture: it fixes the PRNG draw order, so a new family
// is appended rather than inserted. Inserting one would re-draw every sample
// after it and rewrite fixtures that nothing about the change had touched.
for (const family of [encoding, text, digest, crypto, compress, extract, shape, bitwise, classical, checksum, sets, legacyDigest, casing, shaping, unicodeEscape, brute, misc, substitute, netfmt, markup, varint, braille, annotate, bigint, framing, numeric, mail, crosskind, sponge, snort, bacon, legacyHash, bifid, caseregex, unixperms, rc4drop, punycode, bech32, ls47, stats, offsetcheck]) {
    await family.add(builder);
}

const {cases} = builder;
let failures = 0;
for (const testCase of cases) {
    testCase.outputs_hex = [];
    for (let length = 1; length <= testCase.recipe.length; length += 1) {
        try {
            const {hex, html} = await bakeOutput(
                chef,
                makeInput(testCase.input),
                testCase.recipe.slice(0, length),
            );
            // An HTML operation may only be the last step. FerroSift passes
            // its markup on as text; the reference passes the stripped form.
            // Chaining past one would pin a divergence that says nothing about
            // either operation, so it is refused here rather than explained
            // later.
            if (html && length < testCase.recipe.length) {
                throw new Error(
                    `step ${length} produces HTML and is not last; ` +
                        "an HTML operation may only end a recipe",
                );
            }
            testCase.outputs_hex.push(hex);
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
    reference: {name: "CyberChef", version: profile.version, commit: profile.commit},
    cases: complete,
};

await mkdir(path.dirname(output), {recursive: true});
await writeFile(output, `${JSON.stringify(suite, null, 1)}\n`, "utf8");
process.stdout.write(
    `wrote ${complete.length} corpus cases (${failures} bake failures dropped) to ${output}\n`,
);






