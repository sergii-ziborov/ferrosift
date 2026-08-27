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

// The divergence list is read the same way the exemption list is, and for a
// related reason: it names what a status word cannot say on its own.
const divergencesPath = path.join(repoRoot, "docs/compatibility/divergences.json");

/** Reads the recorded behavioural divergences, keyed by alias. */
function divergences() {
    const file = JSON.parse(readFileSync(divergencesPath, "utf8"));
    return new Map(file.divergences.map(entry => [entry.alias, entry]));
}

/**
 * What backs the claim: how the operation was checked, not how well it did.
 *
 * `differential_pinned` is a replayed case count. `pinned_elsewhere` is a
 * hand-written test named by an exemption, for operations the automatic corpus
 * cannot sample. `round_trip` is an operation whose own bytes are one valid
 * encoding among several, checked through the inverse that *is* pinned.
 * `none` means an alias with nothing behind it, which the corpus gate rejects
 * before this file is written — so it is always zero, and counted anyway so
 * that the zero is visible. An operation with no alias is `not_applicable`
 * rather than unevidenced: there is no reference claim to evidence.
 */
function evidenceOf(alias, cases, exemption) {
    if (!alias) return "not_applicable";
    if (cases > 0) return "differential_pinned";
    if (exemption?.scope === "elsewhere") return "pinned_elsewhere";
    if (exemption?.scope === "interoperable") return "round_trip";
    return "none";
}

/**
 * How close the behaviour is, which is a different question from how it was
 * checked.
 *
 * These two were one word until now, and the word was `exact`. It meant "the
 * reference bytes are pinned" and it was read as "this matches the reference
 * everywhere" — a claim no case count can make, because a corpus can only
 * cover the inputs it holds. An operation that is byte-pinned over its whole
 * corpus and refuses one documented class of input outside it is honestly
 * described by neither half alone.
 */
function parityOf(alias, exemption, divergence) {
    if (!alias) return "native";
    if (divergence) return "documented_divergence";
    if (exemption?.scope === "interoperable") return "interoperable";
    return "exact";
}

export function buildLedger() {
    const membership = packMembership();
    const cases = conformanceCases();
    const exempt = exemptions();
    const diverge = divergences();
    const page = readFileSync(
        path.join(repoRoot, `docs/compatibility/cyberchef-v${REFERENCE.version}.md`),
        "utf8",
    );
    const entries = catalog("portable-full").map(operation => {
        const alias =
            operation.aliases.find(entry => entry.profile === "CyberChefV11_3")?.name ?? null;
        const count = alias ? (cases.get(alias) ?? 0) : 0;
        const exemption = alias ? (exempt.get(alias) ?? null) : null;
        const divergence = alias ? (diverge.get(alias) ?? null) : null;
        const entry = {
            id: operation.id,
            display_name: operation.display_name,
            category: operation.category,
            reference_alias: alias,
            feature: membership.get(operation.id) ?? "core",
            targets: operation.targets,
            conformance_cases: count,
            evidence: evidenceOf(alias, count, exemption),
            parity: parityOf(alias, exemption, divergence),
        };
        if (exemption) entry.note = exemption.reason;
        if (divergence) {
            entry.divergence = {domain: divergence.domain, reason: divergence.reason};
        }
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

    // A divergence that names nothing, or a section that is not on the page,
    // is a claim about something that is no longer there.
    for (const [alias, entry] of diverge) {
        if (!aliases.has(alias)) {
            throw new Error(`divergence \`${alias}\` names an operation that is not registered`);
        }
        if (!page.includes(`<a id="${entry.section}"`) && !headingAnchor(page, entry.section)) {
            throw new Error(
                `divergence \`${alias}\` points at section \`${entry.section}\`, `
                    + `which cyberchef-v${REFERENCE.version}.md does not have`,
            );
        }
        if (exempt.has(alias)) {
            throw new Error(
                `\`${alias}\` is both exempt from byte-pinning and recorded as diverging; `
                    + "one of the two is wrong",
            );
        }
    }

    const by = (field, value) => entries.filter(entry => entry[field] === value).length;
    return {
        reference: REFERENCE,
        totals: {
            operations: entries.length,
            aliased: entries.filter(entry => entry.reference_alias).length,
            pinned_cases: [...cases.values()].reduce((sum, value) => sum + value, 0),
            evidence: {
                differential_pinned: by("evidence", "differential_pinned"),
                pinned_elsewhere: by("evidence", "pinned_elsewhere"),
                round_trip: by("evidence", "round_trip"),
                none: by("evidence", "none"),
                not_applicable: by("evidence", "not_applicable"),
            },
            parity: {
                exact: by("parity", "exact"),
                documented_divergence: by("parity", "documented_divergence"),
                interoperable: by("parity", "interoperable"),
                native: by("parity", "native"),
            },
        },
        operations: entries,
    };
}

/** Whether the page has a heading GitHub would give this anchor. */
function headingAnchor(page, anchor) {
    for (const line of page.split(/\r?\n/u)) {
        if (!line.startsWith("#")) continue;
        const slug = line
            .replace(/^#+\s*/u, "")
            .toLowerCase()
            .replace(/[^\w\s-]/gu, "")
            .trim()
            .replace(/\s+/gu, "-");
        if (slug === anchor) return true;
    }
    return false;
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
        `| Pinned cases | ${totals.pinned_cases} |`,
        "",
        "Two questions, asked separately, because one word was answering both",
        "and could only answer one. **Evidence** is how an operation was",
        "checked. **Parity** is how close it came. An operation can be pinned",
        "byte for byte across its whole corpus and still refuse one documented",
        "class of input outside it — a corpus covers the cases it holds, and",
        "cannot speak for the ones it does not.",
        "",
        "| Evidence | |",
        "|---|---:|",
        `| Differential-pinned against the reference | ${totals.evidence.differential_pinned} |`,
        `| Pinned by a named test instead of the corpus | ${totals.evidence.pinned_elsewhere} |`,
        `| Checked through a pinned inverse | ${totals.evidence.round_trip} |`,
        `| No reference claim to evidence | ${totals.evidence.not_applicable} |`,
        `| Aliased with no evidence | ${totals.evidence.none} |`,
        "",
        "| Parity | |",
        "|---|---:|",
        `| Exact | ${totals.parity.exact} |`,
        `| Documented divergence | ${totals.parity.documented_divergence} |`,
        `| Interoperable rather than byte-identical | ${totals.parity.interoperable} |`,
        `| FerroSift-native, no reference to match | ${totals.parity.native} |`,
        "",
        "`Aliased with no evidence` is a build failure rather than a footnote:",
        "an alias with neither pinned bytes nor a recorded reason is refused by",
        "the corpus coverage gate, which reads the same exemption list this",
        "table does. It is counted here so the zero is visible rather than",
        "implied.",
        "",
        "`documented divergence` names an operation that differs from the",
        "reference over a stated domain, listed in `divergences.json` with the",
        "domain, the reason, and the section of the compatibility page that",
        "argues it. Every one of them is byte-pinned over the inputs it covers;",
        "the divergence is what lies outside those inputs.",
        "",
        "| Operation | Alias | Pack | Evidence | Parity | Cases |",
        "|---|---|---|---|---|---:|",
    ];
    for (const entry of ledger.operations) {
        const alias = entry.reference_alias ?? "—";
        const detail = entry.divergence
            ? ` (${entry.divergence.domain})`
            : entry.note
              ? ` (${entry.note})`
              : "";
        lines.push(
            `| \`${entry.id}\` | ${alias} | ${entry.feature} | ${entry.evidence} `
                + `| ${entry.parity}${detail} | ${entry.conformance_cases} |`,
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
        "| Registered operations | CyberChef-aliased | Differential-pinned | Exact parity | Pinned cases |",
        "|---:|---:|---:|---:|---:|",
        `| ${totals.operations} | ${totals.aliased} | ${totals.evidence.differential_pinned} `
            + `| **${totals.parity.exact}** | **${totals.pinned_cases}** |`,
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
            `${ledger.totals.evidence.differential_pinned} differential-pinned, ` +
            `${ledger.totals.parity.exact} exact parity, ` +
            `${ledger.totals.pinned_cases} pinned cases\n`,
    );
}
