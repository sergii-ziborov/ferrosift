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

// `--check` turns the extras from a note into a failure. An alias is the claim
// "this operation is the reference's operation of that name", and an extra is
// that claim made about a name this version of the reference does not have --
// a typo, or an operation tagged with a profile it predates. Neither is visible
// to `cargo test`: the replay gates demand evidence for the aliases a spec
// carries, and a name the reference never had simply has no case to demand.
// The vendored checkout is what can answer it, so this is where it is answered.
if (process.argv.includes("--check")) {
    if (extra.length) {
        process.stderr.write(
            `CyberChef ${profile.version} does not have ${extra.length} of the names ` +
                `FerroSift claims for it\n`,
        );
        process.exit(1);
    }
    process.stdout.write(`every alias claimed for ${profile.version} exists in it\n`);
} else if (process.argv.includes("--json")) {
    process.stdout.write(`${JSON.stringify({missing, extra}, null, 2)}\n`);
} else {
    process.stdout.write(missing.map(name => `${name}\n`).join(""));
}
