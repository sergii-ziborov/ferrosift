// Shared plumbing for the CyberChef reference oracle.
//
// The oracle is a development tool. It never ships inside the FerroSift
// runtime: the crates stay pure Rust with no JavaScript at build or run time.
// Its only job is to produce pinned fixtures that `cargo test` then replays.
import {execFileSync} from "node:child_process";
import path from "node:path";
import {existsSync} from "node:fs";
import {fileURLToPath, pathToFileURL} from "node:url";

/**
 * Every reference this repository can be pinned to.
 *
 * More than one, because a compatibility claim is against a version and not
 * against a project. When upstream changes an operation's behaviour, the
 * honest record is that FerroSift matches 11.3 and differs from 11.4 — which
 * cannot be said at all while the version is a single constant. Versioned
 * operation identifiers already carry the other half of that: a changed
 * operation becomes `@2` rather than silently replacing `@1`.
 *
 * Evidence for an older profile is never discarded when a newer one is added.
 * A caller pinned to 11.3 is entitled to know FerroSift still matches it.
 */
export const PROFILES = {
    "11.3.0": {
        version: "11.3.0",
        commit: "d24ba1afce2e3a080308b5df7db033332fe94a1a",
    },
    "11.4.0": {
        version: "11.4.0",
        // The annotated tag `v11.4.0` points at this commit; the tag object's
        // own SHA is different and would check out nothing useful.
        commit: "49d1a5634a67a3b806c6db0fdca7dcecb41a776c",
    },
};

/** The profile used when none is named. */
export const DEFAULT_PROFILE = "11.3.0";

export const UPSTREAM = "https://github.com/gchq/CyberChef.git";

const here = path.dirname(fileURLToPath(import.meta.url));

/** Repository root, three levels above this file. */
export const repoRoot = path.resolve(here, "../..");

/**
 * Reads the profile from `--profile` or `FERROSIFT_CYBERCHEF_PROFILE`.
 *
 * An unknown name is refused rather than defaulted: silently measuring against
 * a different version than the one asked for is the failure this whole
 * arrangement exists to prevent.
 */
export function selectedProfile(argv = process.argv) {
    const flagIndex = argv.indexOf("--profile");
    const requested =
        (flagIndex >= 0 ? argv[flagIndex + 1] : undefined) ??
        process.env.FERROSIFT_CYBERCHEF_PROFILE ??
        DEFAULT_PROFILE;

    const profile = PROFILES[requested];
    if (!profile) {
        throw new Error(
            `unknown profile ${requested}; known: ${Object.keys(PROFILES).join(", ")}`,
        );
    }
    return profile;
}

/** Where generated fixtures for one profile are written. */
export function fixtureDirFor(profile) {
    return path.join(
        repoRoot,
        "crates/ferrosift-operations/tests/fixtures",
        `cyberchef-v${profile.version}`,
    );
}

/**
 * Resolves the checkout for one profile.
 *
 * `FERROSIFT_CYBERCHEF_DIR` wins when set, so an existing clone can be reused;
 * otherwise the vendored path this tool sets up by default is used. The
 * override is per-invocation, so pointing it at the wrong version is caught by
 * the commit check below rather than producing a mislabelled fixture.
 */
export function checkoutPathFor(profile) {
    const override = process.env.FERROSIFT_CYBERCHEF_DIR;
    if (override) return path.resolve(override);
    return path.join(here, "vendor", `cyberchef-v${profile.version}`);
}

/**
 * Fails unless the checkout sits exactly on the pinned tag and commit.
 *
 * A fixture generated against anything else would not be evidence, so this is
 * a hard failure rather than a warning.
 */
export function verifyCheckout(profile = selectedProfile()) {
    const checkout = checkoutPathFor(profile);
    const tag = `v${profile.version}`;
    if (!existsSync(checkout)) {
        throw new Error(
            `reference checkout missing at ${checkout}\n` +
                `run: cargo xtask cyberchef setup --profile ${profile.version}`,
        );
    }
    const commit = git(checkout, "rev-parse", "HEAD");
    let found = "";
    try {
        found = git(checkout, "describe", "--tags", "--exact-match");
    } catch {
        found = "(untagged)";
    }
    if (commit !== profile.commit || found !== tag) {
        throw new Error(
            `reference mismatch: expected ${tag} ${profile.commit}, found ${found} ${commit}`,
        );
    }
    return checkout;
}

/**
 * Loads the pinned CyberChef node API.
 *
 * `src/node/index.mjs` is generated, not committed: a Grunt task enumerates the
 * operation directory and writes the barrel file. It happens to be present in
 * some checkouts and absent in others, so its absence is reported as the build
 * step it is rather than as a missing module three frames deep.
 */
export async function loadChef(profile = selectedProfile()) {
    const checkout = verifyCheckout(profile);
    const entry = path.join(checkout, "src/node/index.mjs");
    if (!existsSync(entry)) {
        throw new Error(
            `reference node entry missing at ${entry}\n` +
                "it is generated by the reference's own build; run:\n" +
                `    npx grunt node   (in ${checkout})`,
        );
    }
    const {default: chef} = await import(pathToFileURL(entry).href);
    return chef;
}

/** The dish type number the reference uses for HTML. */
const HTML_DISH = 3;

/**
 * Bakes a recipe and returns the output bytes as lower-case hex.
 *
 * An operation that produces HTML is read from the dish's own value rather
 * than through `get`. Every `get` translates by way of an ArrayBuffer, and the
 * HTML dish's conversion to one strips the tags and unescapes the entities --
 * so asking for bytes, or even for a string, returns the markup with the
 * markup taken out. That is a fine thing to paste somewhere; it is not what
 * the operation produced, and pinning it would have let a port that emitted no
 * highlighting at all pass.
 *
 * Reading `value` is safe here only because the dish has not been translated
 * yet: the first `get` mutates it in place.
 */
export async function bakeHex(chef, input, recipe) {
    const result = await chef.bake(input, recipe);
    if (result.type === HTML_DISH) {
        return Buffer.from(await stringBytes(chef, result.value)).toString("hex");
    }
    return Buffer.from(result.get("byteArray")).toString("hex");
}

/**
 * Turns a string into bytes the way the reference turns any other string.
 *
 * Encoding the markup as UTF-8 directly would be the obvious thing and the
 * wrong one: the reference's string-to-bytes conversion emits one byte per
 * code point below 256, so every other case in the corpus is pinned in that
 * convention. Re-baking through an empty recipe borrows that conversion
 * instead of restating it, so the markup cases cannot drift from the rest.
 */
async function stringBytes(chef, value) {
    const dish = await chef.bake(value, []);
    return dish.get("byteArray");
}

/**
 * Bakes once and reports both the output bytes and whether it was markup.
 *
 * The caller needs the second fact to refuse a case that chains *past* an
 * HTML operation. FerroSift carries the markup forward as its text, while the
 * reference hands the next operation the stripped form, so the two part from
 * the second step onwards for a reason that has nothing to do with either
 * operation being wrong. Until the value model tells markup from text, the
 * harness checks that no case depends on the difference rather than trusting
 * whoever writes one to remember.
 */
export async function bakeOutput(chef, input, recipe) {
    const result = await chef.bake(input, recipe);
    if (result.type === HTML_DISH) {
        const bytes = await stringBytes(chef, result.value);
        return {hex: Buffer.from(bytes).toString("hex"), html: true};
    }
    return {hex: Buffer.from(result.get("byteArray")).toString("hex"), html: false};
}

/** Bakes a recipe and returns the output as the reference's string view. */
export async function bakeString(chef, input, recipe) {
    const result = await chef.bake(input, recipe);
    return result.get("string");
}

/** Bakes a recipe and returns the output bytes. */
export async function bakeBytes(chef, input, recipe) {
    const result = await chef.bake(input, recipe);
    return Buffer.from(result.get("byteArray"));
}

/** Converts a declared case input into the value the reference accepts. */
export function makeInput(input) {
    return input.kind === "bytes" ? Buffer.from(input.hex, "hex") : input.value;
}

/**
 * Records the output at every recipe prefix.
 *
 * Checking prefixes is what makes the fixture catch a divergence at the step
 * that caused it rather than at the end of a long recipe.
 */
export async function bakeEveryPrefix(chef, testCase) {
    const outputs = [];
    for (let length = 1; length <= testCase.recipe.length; length += 1) {
        const {hex} = await bakeOutput(
            chef,
            makeInput(testCase.input),
            testCase.recipe.slice(0, length),
        );
        outputs.push(hex);
    }
    return outputs;
}

export function git(cwd, ...args) {
    return execFileSync("git", ["-C", cwd, ...args], {encoding: "utf8"}).trim();
}
