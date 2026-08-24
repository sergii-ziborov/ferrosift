// Migration gap: reference operations FerroSift does not implement yet.
//
// Reads the pinned reference catalog and the FerroSift catalog, then reports
// what is still missing. This is the work list for growing the operation
// catalog, and it is derived rather than hand-maintained so it cannot drift.
import {execFileSync} from "node:child_process";
import {readFileSync} from "node:fs";
import path from "node:path";

import {VERSION, repoRoot, verifyCheckout} from "./reference.mjs";

const checkout = verifyCheckout();

/** Every operation name the pinned reference exposes. */
function referenceOperations() {
    const config = path.join(checkout, "src/core/config/OperationConfig.json");
    const parsed = JSON.parse(readFileSync(config, "utf8"));
    return Object.keys(parsed).sort();
}

/** Every reference alias FerroSift currently registers. */
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
            if (alias.profile === "CyberChefV11_3") aliases.add(alias.name);
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
    `CyberChef ${VERSION}: ${reference.length} operations\n` +
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

if (process.argv.includes("--json")) {
    process.stdout.write(`${JSON.stringify({missing, extra}, null, 2)}\n`);
} else {
    process.stdout.write(missing.map(name => `${name}\n`).join(""));
}
