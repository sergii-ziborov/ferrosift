// Migration gap: reference operations FerroSift does not implement yet.
//
// Reads the pinned reference catalog and the FerroSift catalog, then reports
// what is still missing. This is the work list for growing the operation
// catalog, and it is derived rather than hand-maintained so it cannot drift.
//
// Each missing operation is also given a *blocker class* -- what actually
// stands in the way. `docs/compatibility/not-implemented.md` groups by import,
// and says so: "The grouping below is by *import*, which is a proxy and not the
// thing itself." The proxy hides the two answers that matter most. Nine of the
// missing operations cannot be byte-pinned by anyone, because their output is
// not a function of their input; fifty-six answer with a rendering rather than
// with bytes. Both were filed under a heading that reads as "waiting on work".
import {execFileSync} from "node:child_process";
import {readFileSync, readdirSync} from "node:fs";
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
 * The reference's source for every operation, keyed by the name it registers.
 *
 * `OperationConfig.json` holds no file name, so the mapping is made the other
 * way round: each file declares `this.name`, and at 11.3.0 all 501 of them are
 * found this way. A file that stopped declaring one would drop out silently, so
 * `--check` refuses a missing operation it could not classify.
 */
function referenceSources() {
    const directory = path.join(checkout, "src/core/operations");
    const sources = new Map();
    for (const file of readdirSync(directory)) {
        const source = readFileSync(path.join(directory, file), "utf8");
        const declared = /this\.name\s*=\s*"([^"]+)"/u.exec(source);
        if (declared) sources.set(declared[1], {file, source});
    }
    return sources;
}

/**
 * What actually stands between this operation and a byte-pinned port.
 *
 * Ordered, first match wins, because the classes are not independent: an
 * operation that draws on an image library *and* embeds the current time is
 * blocked by the clock whatever the library situation is, since no corpus can
 * record an answer that changes every time it is asked.
 *
 * Read from the reference's own source at the pinned commit, so it is a first
 * cut and not a verdict. `not-implemented.md` says it already and it holds
 * here: where a listing and the code disagree, the code is right.
 */
const BLOCKERS = [
    {
        class: "nondeterministic",
        why: "output is not a function of input, so no corpus can record it",
        // `Math.random` without the call parenthesis too: `Shuffle` hands the
        // function itself to a shuffler rather than calling it in place, and a
        // pattern that demanded the parenthesis called that operation portable.
        signal: /\bMath\.random\b|crypto\.getRandomValues|\bDate\.now\s*\(|new Date\s*\(\s*\)/u,
    },
    {
        class: "host-capability",
        why: "needs something outside the process: the network, or time to pass",
        signal: /\bfetch\s*\(|XMLHttpRequest|\bWebSocket\b|\bsetTimeout\s*\(/u,
    },
    {
        class: "not-a-byte-answer",
        why: "answers with a rendering or a file list, so matching it is not matching bytes",
        signal: /this\.(?:output|present)Type\s*=\s*"(?:html|File|List<File>)"/u,
    },
    {
        class: "external-library",
        why: "byte-exactness is against a specific JavaScript library",
        imports: specifier => !specifier.startsWith("."),
    },
    {
        class: "reference-internal",
        why: "byte-exactness is against the reference's own internal library",
        imports: specifier => /\/(?:lib|vendor)\//u.test(specifier),
    },
];

function classify(name, sources) {
    const entry = sources.get(name);
    if (!entry) return {class: "unclassified", detail: "no source file declares this name"};
    const {source} = entry;
    for (const blocker of BLOCKERS) {
        if (blocker.signal) {
            const found = blocker.signal.exec(source);
            if (found) return {class: blocker.class, detail: found[0].trim()};
        }
        if (blocker.imports) {
            const specifiers = [...source.matchAll(/^import\s+[^;]*?from\s+"([^"]+)";/gmu)]
                .map(match => match[1])
                .filter(specifier => specifier !== "../Operation.mjs");
            const hit = specifiers.find(blocker.imports);
            if (hit) return {class: blocker.class, detail: hit};
        }
    }
    return {class: "effort", detail: "imports nothing outside the reference's own source"};
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
const sources = referenceSources();
const missing = reference
    .filter(name => !implemented.has(name))
    .map(name => ({name, ...classify(name, sources)}));
const extra = [...implemented].filter(name => !reference.includes(name)).sort();

/** How many are blocked by each thing, most first. */
function blockerCounts() {
    const counts = new Map();
    for (const operation of missing) {
        counts.set(operation.class, (counts.get(operation.class) ?? 0) + 1);
    }
    return [...counts.entries()].sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]));
}

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

/**
 * The blocker counts the page publishes.
 *
 * Checked here rather than by `tools/ledger/not-implemented.mjs`, for the same
 * reason the coverage counts are: the classification is read from the reference
 * checkout, which is not committed, so `cargo test` cannot see it. A table
 * nothing checks is the state this page was already in once, and the numbers on
 * it were wrong for several revisions.
 */
function statedBlockers() {
    const page = path.join(repoRoot, "docs/compatibility/not-implemented.md");
    const text = readFileSync(page, "utf8");
    const rows = new Map();
    // Scoped to this one table by its own header. The external-library table
    // further down has rows of exactly the same shape -- a backquoted name and
    // a count -- and reading both reported every JavaScript package as a
    // blocker class that had vanished.
    let inside = false;
    for (const line of text.split(/\r?\n/u)) {
        if (line.includes("| Blocker | Count | What it means |")) {
            inside = true;
            continue;
        }
        if (!inside) continue;
        if (!line.startsWith("|")) break;
        const row = /^\|\s*`([a-z-]+)`\s*\|\s*(\d+)\s*\|/u.exec(line);
        if (row) rows.set(row[1], Number(row[2]));
    }
    if (rows.size === 0) {
        throw new Error(
            "not-implemented.md no longer publishes a blocker table in the form this check" +
                " reads; update tools/cyberchef-oracle/gap.mjs alongside it",
        );
    }
    return rows;
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

    // An operation whose source could not be found is reported rather than
    // quietly filed as blocked by effort. The mapping goes through
    // `this.name`, and an upstream refactor that stopped declaring one would
    // otherwise make the whole backlog look easier than it is.
    for (const operation of missing.filter(one => one.class === "unclassified")) {
        failures.push(`no source file in the checkout declares "${operation.name}"`);
    }

    // The page describes one profile. Checking it against a different one would
    // report a disagreement that is only a difference of version.
    const stated = statedCoverage();
    if (stated.version === profile.version) {
        const published = statedBlockers();
        const measured = new Map(blockerCounts());
        for (const [blocker, count] of measured) {
            if (published.get(blocker) !== count) {
                failures.push(
                    `not-implemented.md says ${published.get(blocker) ?? "nothing"} operations are` +
                        ` blocked by \`${blocker}\`; ${count} are`,
                );
            }
        }
        for (const blocker of published.keys()) {
            if (!measured.has(blocker)) {
                failures.push(
                    `not-implemented.md lists \`${blocker}\`, which nothing is blocked by now`,
                );
            }
        }
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
    const blocked = missing.map(operation => ({
        name: operation.name,
        blocker_class: operation.class,
        blocker_detail: operation.detail,
    }));
    process.stdout.write(
        `${JSON.stringify({missing: blocked, blockers: blockerCounts(), extra}, null, 2)}\n`,
    );
} else {
    // Grouped rather than alphabetical, because a backlog sorted by name buries
    // the answer it exists to give. The operations blocked by nothing but work
    // are the ones somebody can pick up this afternoon, and there are sixteen
    // of them among two hundred and fifty-two.
    const why = new Map(BLOCKERS.map(blocker => [blocker.class, blocker.why]));
    why.set("effort", "nothing stands in the way but the work");
    why.set("unclassified", "no source file in the checkout declares this name");

    for (const [blocker, count] of blockerCounts()) {
        process.stdout.write(`## ${blocker} (${count}) — ${why.get(blocker) ?? ""}\n`);
        for (const operation of missing.filter(one => one.class === blocker)) {
            process.stdout.write(`  ${operation.name}  [${operation.detail}]\n`);
        }
        process.stdout.write("\n");
    }
}
