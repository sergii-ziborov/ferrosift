// Shared plumbing for the CyberChef reference oracle.
//
// The oracle is a development tool. It never ships inside the FerroSift
// runtime: the crates stay pure Rust with no JavaScript at build or run time.
// Its only job is to produce pinned fixtures that `cargo test` then replays.
import {execFileSync} from "node:child_process";
import path from "node:path";
import {existsSync} from "node:fs";
import {fileURLToPath, pathToFileURL} from "node:url";

/** The exact reference this repository is pinned to. */
export const VERSION = "11.3.0";
export const TAG = `v${VERSION}`;
export const COMMIT = "d24ba1afce2e3a080308b5df7db033332fe94a1a";
export const UPSTREAM = "https://github.com/gchq/CyberChef.git";

const here = path.dirname(fileURLToPath(import.meta.url));

/** Repository root, three levels above this file. */
export const repoRoot = path.resolve(here, "../..");

/** Where generated fixtures are written. */
export const fixtureDir = path.join(
    repoRoot,
    "crates/ferrosift-operations/tests/fixtures",
    `cyberchef-v${VERSION}`,
);

/**
 * Resolves the pinned checkout.
 *
 * `FERROSIFT_CYBERCHEF_DIR` wins when set, so an existing clone can be reused;
 * otherwise the vendored path this tool sets up by default is used.
 */
export function checkoutPath() {
    const override = process.env.FERROSIFT_CYBERCHEF_DIR;
    if (override) return path.resolve(override);
    return path.join(here, "vendor", `cyberchef-v${VERSION}`);
}

/**
 * Fails unless the checkout sits exactly on the pinned tag and commit.
 *
 * A fixture generated against anything else would not be evidence, so this is
 * a hard failure rather than a warning.
 */
export function verifyCheckout() {
    const checkout = checkoutPath();
    if (!existsSync(checkout)) {
        throw new Error(
            `reference checkout missing at ${checkout}\n` +
                `run: cargo xtask cyberchef setup`,
        );
    }
    const commit = git(checkout, "rev-parse", "HEAD");
    let tag = "";
    try {
        tag = git(checkout, "describe", "--tags", "--exact-match");
    } catch {
        tag = "(untagged)";
    }
    if (commit !== COMMIT || tag !== TAG) {
        throw new Error(
            `reference mismatch: expected ${TAG} ${COMMIT}, found ${tag} ${commit}`,
        );
    }
    return checkout;
}

/** Loads the pinned CyberChef node API. */
export async function loadChef() {
    const checkout = verifyCheckout();
    const {default: chef} = await import(
        pathToFileURL(path.join(checkout, "src/node/index.mjs")).href
    );
    return chef;
}

/** Bakes a recipe and returns the output bytes as lower-case hex. */
export async function bakeHex(chef, input, recipe) {
    const result = await chef.bake(input, recipe);
    return Buffer.from(result.get("byteArray")).toString("hex");
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
        outputs.push(
            await bakeHex(
                chef,
                makeInput(testCase.input),
                testCase.recipe.slice(0, length),
            ),
        );
    }
    return outputs;
}

export function git(cwd, ...args) {
    return execFileSync("git", ["-C", cwd, ...args], {encoding: "utf8"}).trim();
}
