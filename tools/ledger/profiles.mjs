// Regenerates the case counts on docs/compatibility/profiles.md.
//
// The page had said 2544 corpus cases and 65 differential cases since the day
// it was written. The fixtures were at 6063 and 79 by then, and had gained a
// third file nobody had told the page about. Numbers in prose go stale exactly
// this way: they are true when typed and nothing ever asks them again.
//
// So the counts come from the fixtures, the same ones the replay reads, and
// `cargo xtask ledger check` fails when the page and the files disagree.

import {readFileSync, writeFileSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../..");

export const profilesPath = path.join(repoRoot, "docs/compatibility/profiles.md");

const BASELINE = "11.3.0";
const COMPARED = "11.4.0";

/** Every fixture family, in the order the page introduces them. */
const FAMILIES = ["corpus", "differential", "flow"];

function fixture(version, name) {
    const file = path.join(
        repoRoot,
        "crates/ferrosift-operations/tests/fixtures",
        `cyberchef-v${version}`,
        name,
    );
    return JSON.parse(readFileSync(file, "utf8"));
}

function counts() {
    const baseline = {};
    const overlay = {};
    for (const family of FAMILIES) {
        baseline[family] = fixture(BASELINE, `${family}.json`).cases.length;
        overlay[family] = fixture(COMPARED, `${family}.overlay.json`);
    }
    return {baseline, overlay};
}

/** A count with a thousands separator, because these are read by people. */
function grouped(value) {
    return value.toLocaleString("en-US");
}

/** The headline: how many recipes the baseline claim is about. */
export function renderHeadline({baseline}) {
    const total = FAMILIES.reduce((sum, family) => sum + baseline[family], 0);
    return [
        `"Compatible with CyberChef" is not a statement anyone can check; "produces`,
        `CyberChef ${BASELINE}'s exact output bytes for these ${grouped(total)} recipes" is.`,
    ];
}

/** What the newer profile actually contributed, per fixture family. */
export function renderDelta({baseline, overlay}) {
    const total = key =>
        FAMILIES.reduce((sum, family) => sum + (overlay[family][key]?.length ?? 0), 0);
    const compared = FAMILIES.reduce((sum, family) => sum + overlay[family].compared_cases, 0);
    const changed = total("changed");
    const added = total("added");
    const removed = total("removed");

    const lines = [
        `${COMPARED} changed nothing this corpus can see: every one of the`,
        `${grouped(compared)} cases it was replayed against produces byte-identical output under`,
        "both references. What it contributed is cases that could not have existed",
        `in ${BASELINE} at all, because the operations they exercise were introduced later.`,
        "",
        "| Fixture | Baseline | Compared | Changed | Added | Removed |",
        "|---|---:|---:|---:|---:|---:|",
    ];
    for (const family of FAMILIES) {
        const delta = overlay[family];
        lines.push(
            `| \`${family}\` | ${grouped(baseline[family])} | ${grouped(delta.compared_cases)} `
                + `| ${delta.changed.length} | ${delta.added.length} | ${delta.removed.length} |`,
        );
    }
    lines.push(
        "",
        `${changed} changed and ${removed} removed, across every fixture. The ${added} added are`,
        "the ones the baseline reference refuses to bake at all.",
    );
    return lines;
}

/** The overlay shape, with the count it actually carries. */
export function renderShape({overlay}) {
    const compared = overlay.corpus.compared_cases;
    return [
        "```json",
        "{",
        `  "reference": {"name": "CyberChef", "version": "${COMPARED}", "commit": "49d1a56…"},`,
        `  "baseline":  {"version": "${BASELINE}", "commit": "d24ba1a…"},`,
        `  "compared_cases": ${compared},`,
        '  "changed": [], "added": [ … ], "removed": []',
        "}",
        "```",
    ];
}

function replaceBlock(current, marker, lines) {
    const begin = `<!-- ${marker}:begin -->`;
    const end = `<!-- ${marker}:end -->`;
    const start = current.indexOf(begin);
    const stop = current.indexOf(end);
    if (start === -1 || stop === -1) {
        throw new Error(`docs/compatibility/profiles.md is missing its ${begin} / ${end} markers`);
    }
    const block = [begin, ...lines, end].join("\n");
    return current.slice(0, start) + block + current.slice(stop + end.length);
}

export function renderProfiles(current) {
    const data = counts();
    let next = replaceBlock(current, "profiles:headline", renderHeadline(data));
    next = replaceBlock(next, "profiles:delta", renderDelta(data));
    next = replaceBlock(next, "profiles:shape", renderShape(data));
    return next;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    const current = readFileSync(profilesPath, "utf8");
    writeFileSync(profilesPath, renderProfiles(current), "utf8");
    process.stdout.write(`wrote ${profilesPath}\n`);
}
