// Fails when the committed ledger no longer matches the code.
//
// The ledger is only evidence if it cannot drift, so CI regenerates it and
// compares. Any difference is a failure with the diff summarised, not a
// silent refresh.
import {readFileSync} from "node:fs";

import {
    buildLedger,
    jsonPath,
    markdownPath,
    readmePath,
    renderMarkdown,
    renderReadme,
} from "./generate.mjs";

function read(file) {
    try {
        return readFileSync(file, "utf8");
    } catch {
        return null;
    }
}

const ledger = buildLedger();
const readme = read(readmePath);

let stale = false;
for (const [file, expected] of [
    [jsonPath, `${JSON.stringify(ledger, null, 2)}\n`],
    [markdownPath, renderMarkdown(ledger)],
    [readmePath, readme === null ? null : renderReadme(ledger, readme)],
]) {
    const actual = read(file);
    if (actual === null) {
        process.stderr.write(`missing: ${file}\n`);
        stale = true;
    } else if (actual !== expected) {
        process.stderr.write(`stale: ${file}\n`);
        stale = true;
    }
}

if (stale) {
    process.stderr.write("run: cargo xtask ledger generate\n");
    process.exit(1);
}

process.stdout.write(
    `ledger current: ${ledger.totals.operations} operations, ` +
        `${ledger.totals.exact} exact, ${ledger.totals.pinned_cases} pinned cases\n`,
);
