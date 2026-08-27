// Turns Criterion's estimates into docs/benchmarks.md.
//
// The numbers come from Criterion's own output rather than from anything this
// script measures, so the report cannot flatter the result: it can only
// present what the run produced, including where FerroSift loses.
import {readFileSync, readdirSync, writeFileSync, existsSync, statSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../..");
const criterionDir = path.join(repoRoot, "bench/target/criterion");
const reportPath = path.join(repoRoot, "docs/benchmarks.md");

/** Reads every `new/estimates.json` under the criterion directory. */
function measurements() {
    if (!existsSync(criterionDir)) {
        throw new Error(`no criterion output at ${criterionDir}; run: cargo xtask bench run`);
    }
    const results = [];
    for (const group of readdirSync(criterionDir)) {
        const groupDir = path.join(criterionDir, group);
        if (!statSync(groupDir).isDirectory()) continue;
        for (const arm of readdirSync(groupDir)) {
            const armDir = path.join(groupDir, arm);
            if (!statSync(armDir).isDirectory()) continue;
            for (const size of readdirSync(armDir)) {
                const estimates = path.join(armDir, size, "new", "estimates.json");
                if (!existsSync(estimates)) continue;
                const parsed = JSON.parse(readFileSync(estimates, "utf8"));
                results.push({
                    group,
                    arm,
                    size: Number(size),
                    nanoseconds: parsed.median.point_estimate,
                    // Criterion's own interval, carried through so a reader
                    // can see the spread rather than trusting one number.
                    low: parsed.median.confidence_interval.lower_bound,
                    high: parsed.median.confidence_interval.upper_bound,
                });
            }
        }
    }
    return results;
}

/** Human-readable duration, at the precision the number justifies. */
function duration(nanoseconds) {
    if (nanoseconds < 1_000) return `${nanoseconds.toFixed(0)} ns`;
    if (nanoseconds < 1_000_000) return `${(nanoseconds / 1_000).toFixed(2)} µs`;
    return `${(nanoseconds / 1_000_000).toFixed(2)} ms`;
}

function bytes(count) {
    if (count < 1024) return `${count} B`;
    if (count < 1024 * 1024) return `${count / 1024} KiB`;
    return `${count / (1024 * 1024)} MiB`;
}

/** Groups measurements by benchmark group, then by size. */
function tabulate(results) {
    const groups = new Map();
    for (const result of results) {
        if (!groups.has(result.group)) groups.set(result.group, new Map());
        const sizes = groups.get(result.group);
        if (!sizes.has(result.size)) sizes.set(result.size, new Map());
        sizes.get(result.size).set(result.arm, result);
    }
    return groups;
}

/**
 * The arm a group is measured against.
 *
 * Where several comparison crates implement the same thing, the *fastest* one
 * is the baseline. Comparing against the slowest available would be choosing
 * the opponent, and a win chosen that way is not a win.
 */
function baselineArm(arms, sizes) {
    const candidates = arms.filter(
        arm =>
            arm.endsWith("-crate") ||
            arm === "primitive-direct" ||
            arm === "execute-each-call",
    );
    if (candidates.length === 0) return null;
    if (candidates.length === 1) return candidates[0];
    // Total time across the sweep decides which comparison crate is quicker.
    const totals = candidates.map(arm => ({
        arm,
        total: [...sizes.values()].reduce(
            (sum, row) => sum + (row.get(arm)?.nanoseconds ?? 0),
            0,
        ),
    }));
    return totals.reduce((a, b) => (a.total <= b.total ? a : b)).arm;
}

/** The FerroSift arm in a group. */
function subjectArm(arms) {
    return (
        arms.find(arm => arm === "ferrosift") ??
        arms.find(arm => arm === "compiled-pipeline") ??
        arms.find(arm => arm === "through-recipe") ??
        null
    );
}

/**
 * Every performance claim FerroSift is entitled to make, derived from the
 * data rather than written down.
 *
 * A claim appears here only where the subject beat the baseline at some size
 * with non-overlapping intervals on a run that was not noisy. Nothing can be
 * added to this list by editing prose — the only way to make a claim is to
 * earn it in a measurement, and the only way to keep it is to keep earning
 * it. Groups with no supported win are listed too, as no claim.
 */
function renderClaims(groups) {
    const rows = [];
    for (const [name, sizes] of [...groups].sort()) {
        const arms = [...new Set([...sizes.values()].flatMap(row => [...row.keys()]))].sort();
        const subject = subjectArm(arms);
        const baseline = baselineArm(arms, sizes);
        if (!subject || !baseline) {
            rows.push([name, "—", "no comparison arm in this group"]);
            continue;
        }
        const wins = [];
        for (const size of [...sizes.keys()].sort((a, b) => a - b)) {
            const row = sizes.get(size);
            if (!row.has(subject) || !row.has(baseline)) continue;
            const stated = verdict(row.get(subject), row.get(baseline));
            if (stated.endsWith("faster")) wins.push({size, stated});
        }
        // A win at exactly one size, with losses either side of it, is not a
        // performance characteristic. Cost here is broadly linear in input, so
        // a real advantage shows up across a range; a single point that beats
        // its neighbours is the baseline having a bad run. Narrow confidence
        // intervals do not rule that out — they describe the repeatability of
        // one measurement, not whether the surrounding ones agree with it.
        if (wins.length === 1 && sizes.size > 2) {
            rows.push([
                name,
                `\`${baseline}\``,
                `**no claim** — faster only at ${bytes(wins[0].size)}, slower either ` +
                    "side; an isolated win is a noisy baseline, not an advantage",
            ]);
            continue;
        }
        rows.push(
            wins.length === 0
                ? [name, `\`${baseline}\``, "**no claim** — loses or ties at every size"]
                : [
                      name,
                      `\`${baseline}\``,
                      wins.map(win => `${win.stated} at ${bytes(win.size)}`).join(", "),
                  ],
        );
    }

    const supported = rows.filter(row => !row[2].startsWith("**no claim**") && row[1] !== "—");
    return [
        "## Claims",
        "",
        "This table is computed from the measurements, not written. A row can",
        "only say *faster* if the subject beat the fastest comparison arm at",
        "that size, on a run that was not noisy, with confidence intervals that",
        "do not overlap — and the win has to hold across more than one size.",
        "There is no way to add a claim here by editing text.",
        "",
        "That last rule was added because the first three were not enough. A",
        "digest measured *faster through a recipe than by calling the primitive",
        "the recipe calls* passed all of them: both intervals were narrow, they",
        "did not overlap, and neither run looked noisy. It was still impossible.",
        "The baseline had simply had a bad run at one size, and nothing about a",
        "confidence interval — which describes how repeatable one measurement",
        "is — could see that the measurements either side of it disagreed.",
        "",
        `Supported claims right now: **${supported.length}** of ${rows.length} groups.`,
        "",
        "| Group | Compared against | Claim |",
        "|---|---|---|",
        ...rows.map(row => `| ${row[0].replace(/_/g, " / ")} | ${row[1]} | ${row[2]} |`),
        "",
        "## Results",
        "",
    ];
}

/**
 * The machine, compiler, and commit behind each batch.
 *
 * Every batch gets a row because they are measured separately. A single
 * commit across the whole report would be a claim nobody could keep once
 * partial re-runs became the normal way to work.
 */
function renderProvenance(batches) {
    const names = Object.keys(batches).sort();
    if (names.length === 0) return [];

    const first = batches[names[0]];
    const lines = [
        "## Measured on",
        "",
        "| | |",
        "|---|---|",
        // `rustc -vV` is multi-line. Take the first line however it was
        // stored — escaped by the recorder, or literal if hand-edited — since
        // a real newline would break the table.
        `| Compiler | ${first.rustc.split(/\\n|\n/)[0].trim()} |`,
        `| Platform | ${first.os} / ${first.arch} |`,
        `| CPU | ${first.cpu} |`,
        "",
        "| Batch | Commit | Working tree |",
        "|---|---|---|",
    ];
    for (const name of names) {
        const batch = batches[name];
        lines.push(
            `| ${name} | \`${(batch.commit ?? "unknown").slice(0, 8)}\` | ` +
                `${batch.dirty ? "**uncommitted changes present**" : "clean"} |`,
        );
    }
    lines.push("");
    return lines;
}

function renderGroup(name, sizes) {
    const arms = [...new Set([...sizes.values()].flatMap(row => [...row.keys()]))].sort();
    const subject = subjectArm(arms);
    const baseline = baselineArm(arms, sizes);

    const header = ["| Size |", ...arms.map(arm => ` \`${arm}\` |`), " verdict |"].join("");
    const divider = ["|---:|", ...arms.map(() => "---:|"), "---|"].join("");
    const lines = [`### ${name.replace(/_/g, " / ")}`, "", header, divider];

    // Every size in the sweep gets a row and a verdict, so a loss is as
    // visible as a win. Summarising only the favourable end is what makes a
    // benchmark look chosen.
    for (const size of [...sizes.keys()].sort((a, b) => a - b)) {
        const row = sizes.get(size);
        const cells = arms.map(arm =>
            row.has(arm) ? ` ${duration(row.get(arm).nanoseconds)} |` : " — |",
        );
        let stated = " — |";
        if (subject && baseline && row.has(subject) && row.has(baseline)) {
            stated = ` ${verdict(row.get(subject), row.get(baseline))} |`;
        }
        lines.push([`| ${bytes(size)} |`, ...cells, stated].join(""));
    }
    lines.push("");

    if (subject && baseline) {
        lines.push(
            `Verdict compares \`${subject}\` against \`${baseline}\`, the fastest`,
            "comparison arm in this group.",
            "",
        );
    }
    lines.push("<details><summary>Confidence intervals</summary>", "");
    lines.push("| Size | Arm | Median | 95% interval |", "|---:|---|---:|---|");
    for (const size of [...sizes.keys()].sort((a, b) => a - b)) {
        for (const arm of arms) {
            const cell = sizes.get(size).get(arm);
            if (!cell) continue;
            lines.push(
                `| ${bytes(size)} | \`${arm}\` | ${duration(cell.nanoseconds)} | ` +
                    `${duration(cell.low)} – ${duration(cell.high)} |`,
            );
        }
    }
    lines.push("", "</details>", "");
    return lines;
}

/** States a ratio plainly, in whichever direction it points. */
function describe(ratio) {
    if (ratio < 1) return `${(1 / ratio).toFixed(2)}× faster`;
    if (ratio > 1) return `${ratio.toFixed(2)}× slower`;
    return "even";
}

/**
 * A measurement whose interval is wide relative to its median.
 *
 * A loaded machine produces numbers that look like results. Marking them
 * stops a reader — or a later version of this file — from treating noise as a
 * finding.
 */
const NOISE_THRESHOLD = 0.15;

function noisy(cell) {
    return (cell.high - cell.low) / cell.nanoseconds > NOISE_THRESHOLD;
}

/**
 * The verdict for one row, or a refusal to give one.
 *
 * A ratio is only stated when the two confidence intervals do not overlap.
 * Where they do, the run cannot tell the two apart and saying which is faster
 * would be reading a preference into noise — which is the failure mode this
 * whole file is arranged against, and it applies in FerroSift's favour
 * exactly as much as against it.
 */
function verdict(subject, baseline) {
    if (noisy(subject) || noisy(baseline)) return "noisy — rerun";
    const overlap = subject.low <= baseline.high && baseline.low <= subject.high;
    if (overlap) return "no measurable difference";
    return describe(subject.nanoseconds / baseline.nanoseconds);
}

export function render(results, environment) {
    const groups = tabulate(results);
    const lines = [
        "# Benchmarks",
        "",
        "Generated by `cargo xtask bench report` from Criterion's own estimates.",
        "Every figure is a median; nothing here is selected, and a result that",
        "goes against FerroSift is printed the same way as one that does not.",
        "",
        "## Why you should believe this",
        "",
        "A benchmark published by the project it flatters is worth nothing",
        "unless it is arranged so that rigging it would be visible. Six things",
        "make that true here.",
        "",
        "**Every size is printed, with a verdict on each row.** There is no",
        "summary that quotes the favourable end of a sweep. If FerroSift loses",
        "at 16 bytes and wins at 1 MiB, both rows say so in the same column.",
        "",
        "**The baseline is the fastest available competitor, not a convenient",
        "one.** Where two crates do the same job — `hex` and `faster-hex` —",
        "both are measured and the quicker one is what the verdict compares",
        "against. This rule was added after an earlier run reported a win over",
        "`hex` while `faster-hex` existed and had not been tried.",
        "",
        "**Competitors are called through their documented fast path**, at",
        "exactly pinned versions, with no settings changed to slow them down.",
        "",
        "**FerroSift is handicapped, not helped.** It runs through its full",
        "public surface — registry lookup, argument resolution, budget checks —",
        "and additionally copies its input on every iteration. Reaching past",
        "that to the raw codec would compare two different amounts of work.",
        "",
        "**The inputs are deterministic and the generator is in this",
        "repository**, so anyone can produce the same bytes and check.",
        "",
        "**The raw estimates are committed** next to this file as",
        "`benchmarks.json`, and the machine and compiler are recorded below. A",
        "reader who does not trust the prose can recompute every ratio from the",
        "data, or re-run the whole thing with one command.",
        "",
        "**A ratio is only stated when the run can support it.** Where the two",
        "confidence intervals overlap the verdict reads *no measurable",
        "difference* rather than picking a direction, and a measurement whose",
        "own interval is wider than 15% of its median is marked *noisy* and",
        "reported as no result at all. This cuts both ways and is meant to: an",
        "earlier run on a loaded machine produced a FerroSift digest that",
        "appeared faster than the primitive it calls, which is impossible, and",
        "nothing in the report at the time would have stopped that being read",
        "as a win.",
        "",
        ...renderProvenance(environment),
        "## Reproducing",
        "",
        "Batches are measured independently, so only what changed needs",
        "re-measuring. At this catalog size a full sweep is minutes; at five",
        "hundred operations it is an afternoon, and an afternoon spent",
        "re-measuring untouched code is an afternoon nobody spends — which is",
        "how published numbers go stale.",
        "",
        "```bash",
        "cargo xtask bench check   # which batches predate a change they cover",
        "cargo xtask bench stale   # re-run only those, then rebuild this file",
        "```",
        "",
        "`cargo xtask bench run encoding` measures one batch by name, and",
        "`cargo xtask bench all` still does everything.",
        "",
        "The comparison against the reference is a separate step, because it",
        "needs the pinned CyberChef checkout that the corpus already uses:",
        "",
        "```bash",
        "cargo xtask bench reference   # measure CyberChef, then rebuild this file",
        "```",
        "",
        "The toolchain is pinned in `bench/rust-toolchain.toml`, every",
        "comparison crate is pinned to an exact version, `bench/Cargo.lock` is",
        "committed, and the optimisation settings are written out in",
        "`bench/Cargo.toml` rather than inherited. Timings will differ between",
        "machines; ratios should not.",
        "",
        "## What is being compared",
        "",
        "Three different questions get three different kinds of arm.",
        "",
        "**Against a specialist crate.** `base64`, `hex`, `crc32fast`, and",
        "`strsim` each do one thing and are among the fastest Rust",
        "implementations of it. Where FerroSift wrote the algorithm itself,",
        "this is a real comparison.",
        "",
        "**Against the primitive.** For the digests and ciphers, FerroSift *is*",
        "RustCrypto — the same code computing the same bytes. Reporting a win",
        "there would be measuring nothing. The useful question is the opposite:",
        "what does going through a recipe cost above calling the primitive",
        "directly? That gap is the library's own overhead and the only part",
        "FerroSift is answerable for.",
        "",
        "**Between entry points.** `execute-each-call` resolves the recipe",
        "against the registry every time; `compiled-pipeline` resolves once.",
        "The difference is what compiling buys.",
        "",
        "FerroSift is always driven through its public surface, arguments and",
        "budget checks included. Reaching past that to the codec would compare",
        "two different amounts of work.",
        "",
        "## Method",
        "",
        "Inputs are a seeded xorshift, so a rerun measures the code rather than",
        "the data. Sizes sweep from 16 B to 1 MiB because per-call overhead",
        "decides the small end and the algorithm decides the large end — a",
        "comparison quoting only one of them is choosing its own answer.",
        "",
        "## Where this stands",
        "",
        "Two findings, and they point opposite ways. Both are stated here",
        "rather than left for a reader to assemble from the tables.",
        "",
        "**FerroSift is not currently faster than any best-in-class specialist",
        "crate, at any size measured.** Every competitive comparison below is a",
        "loss.",
        "",
        "**FerroSift is faster than the reference it ports, by more than an",
        "order of magnitude everywhere it was measured.** That is the weaker",
        "claim of the two and the one worth less: beating a JavaScript",
        "implementation with a Rust one is the least a port should manage, and",
        "it says nothing about whether the Rust is good. The specialist crates",
        "answer that question, and they answer it unfavourably.",
        "",
        "An earlier version of this file claimed a win over the `hex` crate at",
        "64 KiB and above. The claim was arithmetically true and worthless:",
        "`faster-hex` implements the same function with SIMD, had not been",
        "measured, and beats both. Against it FerroSift is between 4.8× and",
        "33.6× slower. The win is retracted, and the rule that produced it —",
        "compare against whichever crate happens to be in the file — has been",
        "replaced by one that compares against the fastest arm in the group.",
        "",
        "Two results are genuine and neither is a competitive claim. Compiling",
        "a pipeline is measurably faster than resolving a recipe on every call,",
        "which is FerroSift against itself and the first evidence that the",
        "compiled path earns its place. And at large inputs the cost of going",
        "through a recipe rather than calling a digest directly is small, which",
        "says the library layer is thin — not that the library is fast.",
        "",
        "The gaps have a shape. Roughly half a microsecond of fixed per-call",
        "cost decides everything below about four kilobytes, which is why the",
        "16-byte rows are so lopsided against crates that are one function.",
        "Above that the algorithms are the measurement, and the ports are",
        "written for exactness against the reference rather than for speed:",
        "they iterate `char` where bytes would do, allocate per chunk, and",
        "carry validation the specialist crates do not.",
        "",
        "None of that is an excuse and none of it is stated as one. They are",
        "the reasons, and each is a thing to fix.",
        "",
        "This harness exists to make that work visible, and it already has. The",
        "first thing it found was a base64 decoder scanning a 64-symbol list",
        "for every character, several times per character; replacing it with a",
        "lookup table made decoding 13 times faster at 1 MiB and cut the gap",
        "from 118× to 7×. The corpus confirmed the output did not change. That",
        "is the loop this file is here to run.",
        "",
        "One caution about the `overhead` rows, learned by re-running them. An",
        "earlier report put the cost of going through a recipe at 1.07× to",
        "1.71× above calling the digest directly, and read that as evidence",
        "the library layer is thin. A second run on the same code could not",
        "reproduce it: most of those rows came back noisy or as no measurable",
        "difference. Gaps that small are at this machine's noise floor, and a",
        "single run cannot settle them. The large gaps in every other group",
        "reproduced without trouble.",
        "",
    ];
    lines.push(...renderReference());
    lines.push(...renderPeer());
    // `renderClaims` closes with the `## Results` heading the group tables sit
    // under, so nothing adds one here -- a second push left an empty heading
    // above the claims table for three revisions.
    lines.push(...renderClaims(groups));
    for (const [name, sizes] of [...groups].sort()) {
        lines.push(...renderGroup(name, sizes));
    }
    // The blank strings above come from the optional environment block.
    return `${lines.filter((line, index) => line !== "" || lines[index - 1] !== "").join("\n")}\n`;
}

/**
 * Renders the comparison against the reference itself.
 *
 * Kept in this file rather than left in the other script's console output,
 * which is where it used to stop. The measurement existed, was careful, and
 * reached nobody: the published document said nothing about the reference for
 * three revisions while `tools/bench/cyberchef.mjs` sat beside it. A number
 * that is not published is not a result, so an absent file now says so here
 * instead of leaving a silence that reads like an absent comparison.
 */
function renderReference() {
    const file = path.join(repoRoot, "docs/benchmarks-cyberchef.json");
    const heading = ["## Against the reference itself", ""];
    if (!existsSync(file)) {
        return [
            ...heading,
            "Not measured for this report. Run `node tools/bench/cyberchef.mjs`",
            "after `cargo xtask bench run`, which needs the pinned CyberChef",
            "checkout. Until then this section is a gap and says so, rather than",
            "reading as though the comparison had been made and gone unmentioned.",
            "",
        ];
    }

    const {rows} = JSON.parse(readFileSync(file, "utf8"));
    const lines = [
        ...heading,
        "Every figure here is a **floor**, not a headline. A row states what",
        "survives reading both sides as unfavourably as the data allows — the",
        "reference at its fastest batch against FerroSift at the slow end of",
        "its interval. The ratio of the medians is larger than the number",
        "printed, and is not printed. Where the two ranges touch at all there",
        "is no verdict, however tight the batches happened to be.",
        "",
        "Node is far noisier than criterion, so a spread is shown wherever the",
        "reference's own batches disagreed by more than 15%. A floor drawn from",
        "noisy batches and one drawn from tight batches are not the same claim",
        "even when they read alike.",
        "",
        "This is the comparison that answers *is the port worth having*. It is",
        "not evidence that the code is fast — the specialist crates below say",
        "it is not — only that a Rust library beats a JavaScript one at the",
        "same work, which is the least one should expect of a port.",
        "",
    ];

    let group = null;
    for (const row of rows) {
        if (row.group !== group) {
            if (group !== null) lines.push("");
            lines.push(
                `### ${row.group}`,
                "",
                "| Size | CyberChef | FerroSift | Verdict |",
                "|---:|---:|---:|---|",
            );
            group = row.group;
        }
        const noise = row.noisy ? ` *(±${(row.spread * 100).toFixed(0)}%)*` : "";
        const said =
            row.verdict?.kind === "faster"
                ? `at least ${row.verdict.ratio.toFixed(1)}× faster${noise}`
                : row.verdict?.kind === "slower"
                  ? `at least ${row.verdict.ratio.toFixed(1)}× slower${noise}`
                  : `*no verdict — the ranges overlap*${noise}`;
        lines.push(
            `| ${bytes(row.size)} | ${duration(row.reference)} | ${
                row.ferrosift === null ? "—" : duration(row.ferrosift)
            } | ${said} |`,
        );
    }
    lines.push("");
    return lines;
}

/**
 * Renders the comparison against the other Rust port.
 *
 * The only arm in this file that is not measured on the machine described
 * above, and the only one that says anything about the *code* rather than
 * about the architecture: rx-chef carries a registry, an operation trait and
 * a pipeline exactly as FerroSift does, so both sides pay for their shape.
 */
function renderPeer() {
    const file = path.join(repoRoot, "docs/benchmarks-peer.json");
    const heading = ["## Against the other Rust port", ""];
    if (!existsSync(file)) {
        return [
            ...heading,
            "Not measured for this report. It needs a platform where unmodified",
            "rx-chef links, which is not Windows — see the note below — and then",
            "`node tools/bench/peer.mjs` to collect what criterion recorded.",
            "",
        ];
    }

    const {rows, revision, measured_on: measuredOn, note} = JSON.parse(readFileSync(file, "utf8"));
    const lines = [
        ...heading,
        `Measured on **${measuredOn}**, against rx-chef at \`${revision.slice(0, 12)}\`,`,
        "unmodified.",
        "",
        `*${note}.*`,
        "",
        "This is the comparison that asks whether a library of *this* shape —",
        "registry, operation trait, typed arguments, pipeline — carries its",
        "structure cheaply. Both sides pay that cost, which is what makes the",
        "answer about the implementations rather than about the architecture.",
        "The specialist crates below cannot answer it: beating `base64` would",
        "mean our codec is good, and losing to it says as much about the",
        "dispatch layer as about the codec.",
        "",
        "A ratio here is the ratio of the medians, not a floor. The gaps are",
        "small enough that the two intervals overlap on many rows, and refusing",
        "a verdict for all of them would hide a result several runs agree on —",
        "so an overlap is marked rather than silently dropped.",
        "",
        "Both arms are timed one after the other in one process, which is the",
        "best available and is not exact: a machine that slows between them",
        "biases the ratio. Two runs of one binary have disagreed here by a",
        "quarter. Read the direction, not the digit.",
        "",
    ];

    let group = null;
    for (const row of rows) {
        if (row.group !== group) {
            if (group !== null) lines.push("");
            lines.push(
                `### ${row.group}`,
                "",
                "| Size | `ferrosift` | `rx-chef` | Ratio |",
                "|---:|---:|---:|---|",
            );
            group = row.group;
        }
        const ratio =
            row.ratio >= 1
                ? `${row.ratio.toFixed(2)}× slower`
                : `${(1 / row.ratio).toFixed(2)}× faster`;
        const caveat = row.overlaps ? " *(intervals overlap)*" : "";
        lines.push(
            `| ${bytes(row.size)} | ${duration(row.ferrosift)} | ${duration(row.peer)} |`
                + ` ${ratio}${caveat} |`,
        );
    }
    lines.push("");
    return lines;
}

/**
 * Reads what each batch was measured on.
 *
 * Batches are measured independently — re-running one leaves the others
 * alone — so provenance is per batch rather than per report. A report that
 * claimed a single commit for all of them would be wrong the moment anyone
 * re-ran less than everything, which at five hundred operations is always.
 */
function provenance() {
    const directory = path.join(repoRoot, "bench/target/provenance");
    if (!existsSync(directory)) return {};
    const batches = {};
    for (const file of readdirSync(directory)) {
        if (!file.endsWith(".json")) continue;
        // Tolerate a byte-order mark: these are small enough to hand-edit, and
        // several Windows tools add one.
        const raw = readFileSync(path.join(directory, file), "utf8").replace(/^﻿/, "");
        batches[file.replace(".json", "")] = JSON.parse(raw);
    }
    return batches;
}

/** The batch a criterion group belongs to, by the bench binary that made it. */
const GROUP_BATCH = {
    base64_encode: "encoding",
    base64_decode: "encoding",
    hex_encode: "encoding",
    checksum_adler32: "digest",
    checksum_crc32: "digest",
    distance_levenshtein: "digest",
    overhead_identity: "dispatch",
    overhead_md5: "dispatch",
    overhead_sha256: "dispatch",
};

if (process.argv[1] === fileURLToPath(import.meta.url)) {
    const results = measurements();
    const recorded = provenance();
    writeFileSync(reportPath, render(results, recorded), "utf8");
    // The raw estimates travel with the report so a reader can recompute
    // every ratio in it rather than taking the prose on trust.
    writeFileSync(
        path.join(repoRoot, "docs/benchmarks.json"),
        `${JSON.stringify({environment: recorded, measurements: results}, null, 1)}\n`,
        "utf8",
    );
    process.stdout.write(`wrote ${results.length} measurements to ${reportPath}\n`);
}
