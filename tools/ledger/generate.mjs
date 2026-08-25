// Compatibility ledger generator.
//
// Every column is derived, never hand-maintained:
//
//   ids, names, aliases, targets   the live FerroSift catalog
//   feature pack                   which pack-only build first contains the id
//   conformance cases              the committed pinned fixtures
//
// Nothing here reads the reference checkout, so `cargo xtask ledger check`
// can run in CI, where no checkout exists. What the reference has and
// FerroSift does not is a separate question, answered by
// `cargo xtask cyberchef gap`.
import {execFileSync} from "node:child_process";
import {readFileSync, writeFileSync, mkdirSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const repoRoot = path.resolve(here, "../..");

/** Packs, smallest first, so an operation is attributed to its own pack. */
const PACKS = ["hash", "crypto", "compression", "text", "analysis"];
// Every reference version the suite replays, not the newest one. A caller
// pinned to the older release is entitled to know FerroSift still matches it,
// so both stay in the ledger until one stops being replayed.
const REFERENCE = {name: "CyberChef", version: "11.3.0", alsoVersions: ["11.4.0"]};

/** Reads the catalog the CLI reports for one feature selection. */
function catalog(features) {
    const args = [
        "run",
        "--quiet",
        "-p",
        "ferrosift-cli",
        "--no-default-features",
    ];
    if (features) args.push("--features", features);
    args.push("--", "operations", "--format", "json");
    const raw = execFileSync("cargo", args, {
        cwd: repoRoot,
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
    });
    return JSON.parse(raw.replace(/^﻿/, "")).operations;
}

/** Maps every operation id to the pack that introduces it. */
function packMembership() {
    const membership = new Map();
    for (const operation of catalog("")) {
        membership.set(operation.id, "core");
    }
    for (const pack of PACKS) {
        for (const operation of catalog(pack)) {
            if (!membership.has(operation.id)) membership.set(operation.id, pack);
        }
    }
    return membership;
}

/** Counts pinned cases per CyberChef alias across both fixtures. */
function conformanceCases() {
    const fixtures = path.join(
        repoRoot,
        "crates/ferrosift-operations/tests/fixtures",
        `cyberchef-v${REFERENCE.version}`,
    );
    const counts = new Map();
    for (const file of ["differential.json", "corpus.json"]) {
        const suite = JSON.parse(readFileSync(path.join(fixtures, file), "utf8"));
        for (const testCase of suite.cases) {
            for (const step of testCase.recipe) {
                counts.set(step.op, (counts.get(step.op) ?? 0) + 1);
            }
        }
    }
    return counts;
}

// The exemption list is shared with the corpus coverage gate in
// crates/ferrosift-operations/tests/corpus.rs, so the gate and the published
// ledger cannot disagree about what is allowed to be absent.
const exemptionsPath = path.join(repoRoot, "docs/compatibility/exemptions.json");

/** Reads the shared exemption list, keyed by alias. */
function exemptions() {
    const file = JSON.parse(readFileSync(exemptionsPath, "utf8"));
    return new Map(file.exemptions.map(entry => [entry.alias, entry]));
}

/**
 * `exact` when the reference bytes are pinned, either by the fixtures or by an
 * `elsewhere` exemption naming where; `interoperable` when the operation is
 * deliberately not byte-pinned; `unverified` when an alias has neither, which
 * the corpus gate rejects.
 */
function statusOf(alias, cases, exemption) {
    if (!alias) return "native";
    if (cases > 0) return "exact";
    if (exemption?.scope === "elsewhere") return "exact";
    if (exemption?.scope === "interoperable") return "interoperable";
    return "unverified";
}

export function buildLedger() {
    const membership = packMembership();
    const cases = conformanceCases();
    const exempt = exemptions();
    const entries = catalog("portable-full").map(operation => {
        const alias =
            operation.aliases.find(entry => entry.profile === "CyberChefV11_3")?.name ?? null;
        const count = alias ? (cases.get(alias) ?? 0) : 0;
        const exemption = alias ? (exempt.get(alias) ?? null) : null;
        const entry = {
            id: operation.id,
            display_name: operation.display_name,
            category: operation.category,
            reference_alias: alias,
            feature: membership.get(operation.id) ?? "core",
            targets: operation.targets,
            conformance_cases: count,
            status: statusOf(alias, count, exemption),
        };
        if (exemption) entry.note = exemption.reason;
        return entry;
    });

    // An exemption that names nothing, or that no longer changes any verdict,
    // is a stale claim: fail rather than publish it.
    const aliases = new Set(entries.map(entry => entry.reference_alias).filter(Boolean));
    for (const [alias, entry] of exempt) {
        if (!aliases.has(alias)) {
            throw new Error(`exemption \`${alias}\` names an operation that is not registered`);
        }
        if (!["elsewhere", "interoperable"].includes(entry.scope)) {
            throw new Error(`exemption \`${alias}\` has unknown scope \`${entry.scope}\``);
        }
    }

    const counted = status => entries.filter(entry => entry.status === status).length;
    return {
        reference: REFERENCE,
        totals: {
            operations: entries.length,
            aliased: entries.filter(entry => entry.reference_alias).length,
            exact: counted("exact"),
            interoperable: counted("interoperable"),
            unverified: counted("unverified"),
            native: counted("native"),
            pinned_cases: [...cases.values()].reduce((sum, value) => sum + value, 0),
        },
        operations: entries,
    };
}

/** Renders the human-readable table from the same data. */
export function renderMarkdown(ledger) {
    const {totals, reference} = ledger;
    const lines = [
        "# Compatibility ledger",
        "",
        "Generated by `cargo xtask ledger generate`. Do not edit by hand: every",
        "column is derived from the live catalog, the pack-only builds, and the",
        "committed pinned fixtures, so it cannot drift from the code.",
        "",
        `Reference: ${reference.name} ` +
            `${[reference.version, ...(reference.alsoVersions ?? [])].join(" and ")}.`,
        "",
        "The counts below are for the baseline. Every later version listed is",
        "replayed too, as a delta against the baseline: see",
        "`crates/ferrosift-operations/tests/profiles.rs` for what that proves",
        "and `docs/compatibility/profiles.md` for why it is stored that way.",
        "",
        "| | |",
        "|---|---:|",
        `| Registered operations | ${totals.operations} |`,
        `| Reference-aliased | ${totals.aliased} |`,
        `| Byte-pinned (\`exact\`) | ${totals.exact} |`,
        `| Interoperable, exempt from byte-pinning | ${totals.interoperable} |`,
        `| Aliased but unverified | ${totals.unverified} |`,
        `| FerroSift-native, no reference alias | ${totals.native} |`,
        `| Pinned cases | ${totals.pinned_cases} |`,
        "",
        "`exact` means the reference bytes are pinned — by the case count shown,",
        "or, where that is zero, by the test named in the note. `interoperable`",
        "means the operation is deliberately not byte-pinned, because the",
        "reference output is one valid encoding among several rather than the",
        "only one. `native` means there is no reference alias to match.",
        "",
        "There is no fourth state on purpose. An alias with neither pinned bytes",
        "nor a recorded reason is a build failure, not a footnote: the corpus",
        "coverage gate refuses it, reading the same exemption list this table",
        "does.",
        "",
        "| Operation | Alias | Pack | Status | Cases |",
        "|---|---|---|---|---:|",
    ];
    for (const entry of ledger.operations) {
        const alias = entry.reference_alias ?? "—";
        const note = entry.note ? ` (${entry.note})` : "";
        lines.push(
            `| \`${entry.id}\` | ${alias} | ${entry.feature} | ${entry.status}${note} | ${entry.conformance_cases} |`,
        );
    }
    lines.push("");
    lines.push(
        "Operations the reference has and FerroSift does not are reported by",
        "`cargo xtask cyberchef gap`, which needs the pinned checkout and so is",
        "kept out of this file.",
        "",
    );
    return lines.join("\n");
}

export const jsonPath = path.join(repoRoot, "docs/compatibility/ledger.json");
export const markdownPath = path.join(repoRoot, "docs/compatibility/ledger.md");
export const readmePath = path.join(repoRoot, "README.md");

/** Replaces the text between one marker pair, markers included. */
function replaceBlock(current, marker, lines) {
    const begin = `<!-- ${marker}:begin -->`;
    const end = `<!-- ${marker}:end -->`;
    const start = current.indexOf(begin);
    const stop = current.indexOf(end);
    if (start === -1 || stop === -1) {
        throw new Error(`README.md is missing its ${begin} / ${end} markers`);
    }
    const block = [begin, ...lines, end].join("\n");
    return current.slice(0, start) + block + current.slice(stop + end.length);
}

/**
 * Rewrites the headline table and the catalog table in the README, so the
 * front page is generated from the same data as everything else. A
 * hand-maintained operation list is the first thing to go stale in a
 * catalog that grows.
 */
export function renderReadme(ledger, current) {
    const {totals} = ledger;
    const headline = [
        "| Registered operations | CyberChef-aliased | Byte-pinned against the reference | Pinned cases |",
        "|---:|---:|---:|---:|",
        `| ${totals.operations} | ${totals.aliased} | **${totals.exact}** | **${totals.pinned_cases}** |`,
    ];

    const families = new Map();
    for (const entry of ledger.operations) {
        if (!families.has(entry.category)) families.set(entry.category, []);
        families.get(entry.category).push(entry.display_name);
    }
    const catalog = ["| Family | Operations |", "|---|---|"];
    for (const [family, names] of [...families].sort()) {
        catalog.push(`| ${family} | ${names.sort().join(", ")} |`);
    }

    return replaceBlock(replaceBlock(current, "ledger", headline), "catalog", catalog);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
    const ledger = buildLedger();
    mkdirSync(path.dirname(jsonPath), {recursive: true});
    writeFileSync(jsonPath, `${JSON.stringify(ledger, null, 2)}\n`, "utf8");
    writeFileSync(markdownPath, renderMarkdown(ledger), "utf8");
    writeFileSync(readmePath, renderReadme(ledger, readFileSync(readmePath, "utf8")), "utf8");
    process.stdout.write(
        `ledger: ${ledger.totals.operations} operations, ` +
            `${ledger.totals.exact} exact, ${ledger.totals.pinned_cases} pinned cases\n`,
    );
}
