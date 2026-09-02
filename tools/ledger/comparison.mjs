// Regenerates the numbers on docs/comparison.md from the committed data.
//
// The page is mostly prose, because "which of these should I use" is a
// judgement and not a measurement. What it must not do is state a number that
// has stopped being true, so every figure on it comes from here: the catalog
// counts from the ledger, the speed ranges from the two comparison fixtures the
// benchmark harness writes.
//
// Facts about the *other* projects are constants below rather than derived,
// each with the revision it was read at. That is the honest arrangement: this
// repository measures FerroSift continuously and read the others once, and a
// generated-looking number would hide the difference.

import {readFileSync, writeFileSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../..");

export const comparisonPath = path.join(repoRoot, "docs/comparison.md");

/**
 * What the other projects say about themselves, and where.
 *
 * Read once, at a pinned revision, from the project's own words. Nothing here
 * is inferred and nothing is measured: this repository has a corpus for
 * FerroSift and none for anyone else, and quoting a count is the most it can
 * honestly say about a catalog it has not replayed.
 */
const OTHERS = {
    rxchef: {
        name: "rx-chef",
        // The same revision `bench/Cargo.toml` pins for the peer benchmark, so
        // the catalog quoted here and the timings on docs/benchmarks.md are the
        // same build.
        revision: "99e26de96e28",
        operations: 478,
        source: "its own README",
    },
    cyberchef: {
        name: "CyberChef 11.3.0",
        revision: "d24ba1af",
        operations: 501,
        source: "`cargo xtask cyberchef gap`",
    },
};

function readJson(relative) {
    return JSON.parse(readFileSync(path.join(repoRoot, relative), "utf8"));
}

/**
 * A ratio, printed at the precision it can carry.
 *
 * Two decimals below ten, one above. Rounding 1.97 to `2×` reads as a rounder
 * number than the measurement is, and a comparison page is the last place to
 * flatter a digit in either direction.
 */
function ratio(value) {
    return value < 10 ? value.toFixed(2) : value.toFixed(1);
}

/** The catalog sizes, side by side, each with where its number came from. */
export function renderScale(ledger) {
    const {totals} = ledger;
    return [
        "| Project | Operations | Counted from |",
        "|---|---:|---|",
        `| FerroSift | ${totals.operations} | this repository's catalog, on every CI run |`,
        `| ${OTHERS.rxchef.name} | ${OTHERS.rxchef.operations} | `
            + `${OTHERS.rxchef.source}, at \`${OTHERS.rxchef.revision}\` |`,
        `| ${OTHERS.cyberchef.name} | ${OTHERS.cyberchef.operations} | `
            + `${OTHERS.cyberchef.source}, at \`${OTHERS.cyberchef.revision}\` |`,
        "",
        `Of the reference's ${OTHERS.cyberchef.operations}, FerroSift has `
            + `${totals.aliased - nativeOnly(ledger)}; `
            + "[not-implemented.md](compatibility/not-implemented.md) owns that number and "
            + "groups the rest by what each is waiting on.",
    ];
}

/**
 * Operations with a reference name the baseline profile never had.
 *
 * The alias total counts every reference name the catalog claims, including
 * three the newer profile introduced. Coverage of 11.3.0's catalog is the total
 * minus those, which is the same subtraction `not-implemented.mjs` makes.
 */
function nativeOnly(ledger) {
    return ledger.operations.filter(
        operation => operation.reference_alias && operation.reference_since !== ledger.reference.version,
    ).length;
}

/** The range of the floors measured against the reference. */
export function renderReference(rows) {
    const ratios = rows.map(row => row.verdict.ratio).filter(value => Number.isFinite(value));
    const groups = [...new Set(rows.map(row => row.group))];
    const lowest = Math.min(...ratios);
    const highest = Math.max(...ratios);
    return [
        `Across ${rows.length} measurements in ${groups.length} groups, every one a floor: `
            + `**at least ${ratio(lowest)}× to ${ratio(highest)}× faster**.`,
        "",
        "A floor, not a headline. Each row reads both sides as unfavourably as "
            + "the data allows — the reference at its fastest batch against "
            + "FerroSift at the slow end of its interval — so the ratio of the "
            + "medians is larger than the number printed and is not printed. "
            + "The per-group tables are in "
            + "[benchmarks.md](benchmarks.md#against-the-reference-itself).",
    ];
}

/** How the peer comparison came out, per group. */
export function renderPeer(peer) {
    const groups = [...new Set(peer.rows.map(row => row.group))];
    const lines = [
        `Measured on ${peer.measured_on}, against ${peer.peer} at `
            + `\`${peer.revision.slice(0, 12)}\`, unmodified.`,
        "",
        "| Group | Sizes measured | How it came out |",
        "|---|---:|---|",
    ];
    for (const group of groups) {
        const rows = peer.rows.filter(row => row.group === group);
        const faster = rows.filter(row => row.ratio < 1).length;
        const slower = rows.filter(row => row.ratio > 1).length;
        const best = Math.min(...rows.map(row => row.ratio));
        const worst = Math.max(...rows.map(row => row.ratio));
        const summary = describe(faster, slower, best, worst, rows.length);
        lines.push(`| \`${group}\` | ${rows.length} | ${summary} |`);
    }
    lines.push(
        "",
        "Read the direction, not the digit: both arms are timed one after the "
            + "other in one process, and two runs of that binary have disagreed "
            + "by a quarter. The per-size rows are in "
            + "[benchmarks.md](benchmarks.md#against-the-other-rust-port).",
    );
    return lines;
}

function describe(faster, slower, best, worst, total) {
    if (faster === total) {
        return `faster at every size, by ${ratio(1 / worst)}× to ${ratio(1 / best)}×`;
    }
    if (slower === total) {
        return `slower at every size, by ${ratio(best)}× to ${ratio(worst)}×`;
    }
    return `mixed — faster at ${faster} of ${total} sizes, `
        + `between ${ratio(1 / best)}× faster and ${ratio(worst)}× slower`;
}

/** Replaces the text between one marker pair, markers included. */
function replaceBlock(current, marker, lines) {
    const begin = `<!-- ${marker}:begin -->`;
    const end = `<!-- ${marker}:end -->`;
    const start = current.indexOf(begin);
    const stop = current.indexOf(end);
    if (start === -1 || stop === -1) {
        throw new Error(`docs/comparison.md is missing its ${begin} / ${end} markers`);
    }
    const block = [begin, ...lines, end].join("\n");
    return current.slice(0, start) + block + current.slice(stop + end.length);
}

/** Rewrites every generated block on the page. */
export function renderComparison(current) {
    const ledger = readJson("docs/compatibility/ledger.json");
    const reference = readJson("docs/benchmarks-cyberchef.json");
    const peer = readJson("docs/benchmarks-peer.json");

    let next = replaceBlock(current, "comparison:scale", renderScale(ledger));
    next = replaceBlock(next, "comparison:reference", renderReference(reference.rows));
    next = replaceBlock(next, "comparison:peer", renderPeer(peer));
    return next;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    const current = readFileSync(comparisonPath, "utf8");
    writeFileSync(comparisonPath, renderComparison(current), "utf8");
    process.stdout.write(`wrote ${comparisonPath}\n`);
}
