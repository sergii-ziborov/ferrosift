// Checks that the not-implemented page still describes the catalog.
//
// The page groups every operation FerroSift does not have under one of three
// headings, and states counts for each. Nothing enforced those counts, and
// they drifted: two operations -- LZNT1 Decompress and Parse TLV -- stayed
// listed as missing for several revisions after they were implemented, and a
// third was double-listed. The numbers looked authoritative and were not.
//
// So the page now claims that its three headings *partition* the missing set,
// and this is what makes that claim cost something. It needs no reference
// checkout: the ledger says what is implemented and how many operations the
// reference has, and the page says the rest.

import {readFileSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../..");
const page = path.join(repoRoot, "docs/compatibility/not-implemented.md");
const ledgerJson = path.join(repoRoot, "docs/compatibility/ledger.json");

/** Splits a comma-separated run of operation names. */
function names(text) {
    return text
        .split(",")
        .map(name => name.trim())
        .filter(Boolean);
}

/**
 * Reads the three groups the page divides the missing operations into.
 *
 * Each is shaped differently on the page, so each is read differently: the
 * external libraries are a table whose third column lists the operations, the
 * internal ones are a table whose first column names one apiece, and the
 * reachable ones are a single comma-separated paragraph.
 */
function groups(markdown) {
    const lines = markdown.split(/\r?\n/u);
    const external = [];
    const internal = [];
    const reachableLines = [];

    // The external table has no heading of its own -- it follows the ranked
    // summary, whose rows have the same shape and whose last column is prose.
    // So it starts at its own header rather than at a row pattern, or the
    // commas in a verdict would be read as operation names.
    let section = null;
    for (const line of lines) {
        if (line.startsWith("## ")) {
            if (line.includes("Blocked through an internal library")) section = "internal";
            else if (line.includes("Reachable without any port")) section = "reachable";
            else section = null;
            continue;
        }
        if (line.includes("| Library | Count | Operations |")) {
            section = "external";
            continue;
        }

        if (section === "external") {
            // The first data row is glued to the separator row, so rows are
            // matched anywhere in the line rather than anchored to its start.
            for (const row of line.matchAll(/\|\s*`[^`]+`\s*\|\s*\d+\s*\|([^|]*)\|/gu)) {
                external.push(...names(row[1]));
            }
        }
        if (section === "internal") {
            const row = /^\|\s*([^|]+?)\s*\|\s*`/u.exec(line);
            if (row) internal.push(row[1].trim());
        }
        if (section === "reachable" && line.includes(",") && !line.startsWith("|")) {
            reachableLines.push(line);
        }
    }

    // The reachable operations are one comma-separated paragraph, and the
    // prose around them also has commas. The list is the longest line in the
    // section by a wide margin, which is a duller rule than trying to tell
    // prose from names and does not break when the prose is rewritten.
    const longest = reachableLines.reduce((best, line) => (line.length > best.length ? line : best), "");
    return {external, internal, reachable: names(longest)};
}

/** Every count the page states about itself, so prose and lists must agree. */
function claims(markdown) {
    const covered = /covers (\d+) of CyberChef [^']+'s (\d+) catalog operations/u.exec(markdown);
    const remaining = /records what the other (\d+) are waiting on/u.exec(markdown);
    const partition = /partition the \d+ exactly: (\d+) plus (\d+) plus (\d+)/u.exec(markdown);
    // Both the internal and the reachable heading open with the same words,
    // so the trailing adverb is what tells them apart.
    const reachable = /These (\d+) import nothing outside the reference's own source, transitively/u
        .exec(markdown);
    if (!covered || !remaining || !partition || !reachable) {
        throw new Error(
            "the page no longer states its counts in the form this check reads;"
                + " update tools/ledger/not-implemented.mjs alongside it",
        );
    }
    return {
        covered: Number(covered[1]),
        catalog: Number(covered[2]),
        remaining: Number(remaining[1]),
        partition: [Number(partition[1]), Number(partition[2]), Number(partition[3])],
        reachable: Number(reachable[1]),
    };
}

const markdown = readFileSync(page, "utf8");
const ledger = JSON.parse(readFileSync(ledgerJson, "utf8"));

// Only the baseline reference's operations count here, because that is the
// catalog the page divides up. An operation a later reference introduced is
// not one of the baseline's 501 and subtracting it from them would report a
// missing operation as covered -- while the page's three groups, which list
// baseline operations, would still add up to the old number and disagree.
//
// What that leaves unsaid is said elsewhere rather than left out: `cargo xtask
// cyberchef gap --profile 11.4.0` reports the newer catalog's own missing set,
// and the ledger marks each later arrival with the version it came from.
const implemented = new Set(
    ledger.operations
        .filter(operation => operation.reference_since === ledger.reference.version)
        .map(operation => operation.reference_alias)
        .filter(Boolean),
);
const laterProfile = ledger.operations.filter(
    operation => operation.reference_alias && operation.reference_since !== ledger.reference.version,
);

const {external, internal, reachable} = groups(markdown);
const stated = claims(markdown);
const failures = [];

// Nothing may be listed as missing that the catalog already has. This is the
// check that would have caught the two stale entries.
for (const [group, listed] of [
    ["external", external],
    ["internal", internal],
    ["reachable", reachable],
]) {
    for (const name of listed) {
        if (implemented.has(name)) {
            failures.push(`${group}: "${name}" is implemented and still listed as missing`);
        }
    }
}

// Nothing may be listed twice, within a group or across two.
const seen = new Map();
for (const [group, listed] of [
    ["external", external],
    ["internal", internal],
    ["reachable", reachable],
]) {
    for (const name of listed) {
        const first = seen.get(name);
        if (first) failures.push(`"${name}" is listed in both ${first} and ${group}`);
        else seen.set(name, group);
    }
}

// The lists must match the counts the page states about them.
const sizes = [external.length, internal.length, reachable.length];
for (const [index, group] of ["external", "internal", "reachable"].entries()) {
    if (sizes[index] !== stated.partition[index]) {
        failures.push(
            `the page says ${stated.partition[index]} ${group} operations and lists ${sizes[index]}`,
        );
    }
}
if (stated.reachable !== reachable.length) {
    failures.push(
        `the reachable heading says ${stated.reachable} and the list holds ${reachable.length}`,
    );
}

// And the whole page must agree with the ledger about how much is left.
//
// Coverage counts *aliases*, not registrations. The two differ by FerroSift's
// own operations, which have no reference name and so cover nothing in
// CyberChef's catalog -- and reading `totals.operations` here made the page
// claim two operations of CyberChef's that it does not have. Nothing noticed,
// because the catalog size on the page was inflated by the same two and every
// other number on it stayed consistent. That size is checked where the catalog
// exists, by `cargo xtask cyberchef gap --check`.
const missing = stated.catalog - implemented.size;
if (stated.covered !== implemented.size) {
    failures.push(
        `the page says ${stated.covered} operations are covered; the ledger has `
            + `${implemented.size} reference aliases across ${ledger.totals.operations} `
            + `registered operations`,
    );
}
if (stated.remaining !== missing) {
    failures.push(`the page says ${stated.remaining} remain; the ledger implies ${missing}`);
}
const total = sizes.reduce((sum, size) => sum + size, 0);
if (total !== missing) {
    failures.push(`the three groups list ${total} operations; ${missing} are missing`);
}

if (failures.length) {
    process.stderr.write(`${page} no longer describes the catalog:\n`);
    for (const failure of failures) process.stderr.write(`  ${failure}\n`);
    process.exit(1);
}

process.stdout.write(
    `not-implemented current: ${total} missing = ${sizes[0]} external + ${sizes[1]} internal`
        + ` + ${sizes[2]} reachable\n`,
);
// Reported rather than checked. These operations are outside the catalog this
// page partitions, and printing the count is what keeps them from being
// invisible simply because the page has no column for them.
if (laterProfile.length) {
    process.stdout.write(
        `not-implemented note: ${laterProfile.length} operation(s) outside `
            + `${ledger.reference.version}: `
            + laterProfile
                .map(operation => `${operation.reference_alias} (${operation.reference_since})`)
                .join(", ")
            + "\n",
    );
}
