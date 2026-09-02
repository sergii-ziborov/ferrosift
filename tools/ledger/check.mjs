// Fails when the committed ledger no longer matches the code.
//
// The ledger is only evidence if it cannot drift, so CI regenerates it and
// compares. Any difference is a failure with the diff summarised, not a
// silent refresh.
import {readFileSync} from "node:fs";

import {comparisonPath, renderComparison} from "./comparison.mjs";
import {profilesPath, renderProfiles} from "./profiles.mjs";
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
// The comparison page is mostly prose with generated figures in it, so it is
// re-rendered from its own current text the same way the README is. A number
// that has stopped being true is the failure this catches; a rewritten
// paragraph is not.
const comparison = read(comparisonPath);
const profiles = read(profilesPath);

let stale = false;
for (const [file, expected] of [
    [jsonPath, `${JSON.stringify(ledger, null, 2)}\n`],
    [markdownPath, renderMarkdown(ledger)],
    [readmePath, readme === null ? null : renderReadme(ledger, readme)],
    [comparisonPath, comparison === null ? null : renderComparison(comparison)],
    [profilesPath, profiles === null ? null : renderProfiles(profiles)],
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
        `${ledger.totals.evidence.differential_pinned} differential-pinned, ` +
        `${ledger.totals.parity.exact} exact parity, ` +
        `${ledger.totals.pinned_cases} pinned cases\n`,
);
