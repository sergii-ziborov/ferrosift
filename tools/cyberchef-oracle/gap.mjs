// Migration gap: reference operations FerroSift does not implement yet.
//
// Reads the pinned reference catalog and the FerroSift catalog, then reports
// what is still missing. This is the work list for growing the operation
// catalog, and it is derived rather than hand-maintained so it cannot drift.
import {execFileSync} from "node:child_process";
import {readFileSync} from "node:fs";
import path from "node:path";

import {repoRoot, selectedProfile, verifyCheckout} from "./reference.mjs";

const profile = selectedProfile();
const checkout = verifyCheckout(profile);

/** Every operation name the pinned reference exposes. */
function referenceOperations() {
    const config = path.join(checkout, "src/core/config/OperationConfig.json");
    const parsed = JSON.parse(readFileSync(config, "utf8"));
    return Object.keys(parsed).sort();
}

/**
 * Every alias FerroSift registers *for this profile*.
 *
 * Reading one fixed profile here was wrong as soon as there were two: asking
 * for the 11.4 gap compared 11.4's catalog against the names claimed in 11.3,
 * so an operation that only exists in the newer reference could not be counted
 * as implemented and one that only exists in the older could not be counted as
 * missing. The comparison has to stay within one version on both sides.
 */
function ferrosiftAliases() {
    const raw = execFileSync(
        "cargo",
        ["run", "--quiet", "-p", "ferrosift-cli", "--", "operations", "--format", "json"],
        {cwd: repoRoot, encoding: "utf8", maxBuffer: 32 * 1024 * 1024},
    );
    const catalog = JSON.parse(raw);
    const aliases = new Set();
    for (const operation of catalog.operations) {
        for (const alias of operation.aliases) {
            if (alias.profile === profile.alias) aliases.add(alias.name);
        }
    }
    return aliases;
}

const reference = referenceOperations();
const implemented = ferrosiftAliases();
const missing = reference.filter(name => !implemented.has(name));
const extra = [...implemented].filter(name => !reference.includes(name)).sort();

const percent = ((implemented.size / reference.length) * 100).toFixed(1);
process.stdout.write(
    `CyberChef ${profile.version}: ${reference.length} operations\n` +
        `FerroSift: ${implemented.size} aliased (${percent}%)\n` +
        `missing: ${missing.length}\n\n`,
);

if (extra.length) {
    process.stdout.write(
        `aliases not present in the reference catalog (check for typos):\n` +
            extra.map(name => `  ${name}\n`).join("") +
            "\n",
    );
}

/**
 * What `not-implemented.md` says it covers, and of how large a catalog.
 *
 * The page names a version and two counts, and nothing in `cargo test` can
 * check either against the thing they describe -- the reference checkout is not
 * committed. Both were wrong for several revisions: each was inflated by the
 * two FerroSift-native operations, which have no reference alias and cover
 * nothing upstream, and inflating both kept every other number on the page
 * consistent. The page's own check reads it against itself, so it saw nothing.
 */
function statedCoverage() {
    const page = path.join(repoRoot, "docs/compatibility/not-implemented.md");
    const text = readFileSync(page, "utf8");
    const claim =
        /covers (\d+) of CyberChef (\d+\.\d+\.\d+)'s (\d+) catalog operations/u.exec(text);
    if (!claim) {
        throw new Error(
            "not-implemented.md no longer states its coverage in the form this check reads;" +
                " update tools/cyberchef-oracle/gap.mjs alongside it",
        );
    }
    return {covered: Number(claim[1]), version: claim[2], catalog: Number(claim[3])};
}

// `--check` turns the extras from a note into a failure, and holds the page's
// counts against the catalog they are about. An alias is the claim "this
// operation is the reference's operation of that name", and an extra is that
// claim made about a name this version of the reference does not have -- a
// typo, or an operation tagged with a profile it predates. None of this is
// visible to `cargo test`: the replay gates demand evidence for the aliases a
// spec carries, and a name the reference never had simply has no case to
// demand. The vendored checkout is what can answer it, so this is where it is
// answered.
if (process.argv.includes("--check")) {
    const failures = extra.map(
        name => `CyberChef ${profile.version} has no operation named "${name}"`,
    );

    // The page describes one profile. Checking it against a different one would
    // report a disagreement that is only a difference of version.
    const stated = statedCoverage();
    if (stated.version === profile.version) {
        if (stated.catalog !== reference.length) {
            failures.push(
                `not-implemented.md says ${profile.version} has ${stated.catalog} operations;` +
                    ` it has ${reference.length}`,
            );
        }
        if (stated.covered !== implemented.size) {
            failures.push(
                `not-implemented.md says ${stated.covered} are covered;` +
                    ` ${implemented.size} carry a ${profile.version} alias`,
            );
        }
    }

    if (failures.length) {
        for (const failure of failures) process.stderr.write(`  ${failure}\n`);
        process.exit(1);
    }
    process.stdout.write(
        `every alias claimed for ${profile.version} exists in it` +
            (stated.version === profile.version ? `, and the page's counts match` : "") +
            "\n",
    );
} else if (process.argv.includes("--json")) {
    process.stdout.write(`${JSON.stringify({missing, extra}, null, 2)}\n`);
} else {
    process.stdout.write(missing.map(name => `${name}\n`).join(""));
}
