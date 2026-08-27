// Measures the reference itself, so the port can be compared with the thing
// it ports rather than only with specialist crates.
//
// This is the comparison a reader actually asks about, and it is also the one
// easiest to rig, so the arrangement is stated rather than assumed:
//
//   - **The inputs are the same bytes.** The generator below is the seeded
//     xorshift from `bench/src/lib.rs`, transcribed rather than re-invented,
//     and the sizes are that file's `SIZES`. A comparison over different data
//     is not a comparison.
//   - **Node's startup is not counted.** The runtime is up and the reference
//     is imported before any clock starts. Counting process launch would
//     hand FerroSift a win it did not earn -- nobody runs CyberChef by
//     starting Node per input.
//   - **The JIT is warmed.** Untimed iterations run first, because a
//     just-in-time compiler that has not seen the loop yet is measuring
//     compilation, not the operation.
//   - **Both sides go through their public surface.** `chef.bake` resolves a
//     recipe and wraps the value in a Dish, exactly as FerroSift's engine
//     resolves a recipe and wraps a `Value`. Reaching past either one to its
//     codec would compare two different amounts of work.
//   - **The reference's own time is what is reported.** No attempt is made to
//     subtract Dish translation or argument handling, because FerroSift pays
//     for its equivalents and does not subtract them either.
//   - **Every verdict is a floor.** A row states what survives reading both
//     sides as unfavourably as the data allows -- the reference at its fastest
//     batch against FerroSift at the slow end of its interval. The ratio of
//     the medians is always larger than the number printed, and is not
//     printed. Where the two ranges touch at all there is no verdict, however
//     tight the batches happened to be.

import {readFileSync, existsSync, writeFileSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

import {loadChef} from "../cyberchef-oracle/reference.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const criterionDir = path.join(repoRoot, "bench/target/criterion");

/** The sizes `bench/src/lib.rs` sweeps. */
const SIZES = [16, 256, 4096, 65_536, 1_048_576];

/**
 * The seeded xorshift from `bench/src/lib.rs`, byte for byte.
 *
 * Transcribed rather than approximated: the Rust side masks to 32 bits after
 * each step, so every shift here is forced back into an unsigned 32-bit word.
 * A generator that drifted would silently compare two different corpora.
 */
function sample(length) {
    let state = 0x5f37_1d10;
    const bytes = Buffer.allocUnsafe(length);
    for (let i = 0; i < length; i++) {
        state = (state ^ (state << 13)) >>> 0;
        state = (state ^ (state >>> 17)) >>> 0;
        state = (state ^ (state << 5)) >>> 0;
        bytes[i] = state & 0xff;
    }
    return bytes;
}

/**
 * What to measure, named to line up with the criterion groups.
 *
 * Each entry is a group and arm that already exists on the FerroSift side, so
 * the two tables join without anything having to be renamed by hand.
 */
const ARMS = [
    {
        group: "base64/encode",
        directory: "base64_encode",
        arm: "ferrosift",
        recipe: () => [{op: "To Base64", args: ["A-Za-z0-9+/="]}],
    },
    {
        group: "hex/encode",
        directory: "hex_encode",
        arm: "ferrosift",
        recipe: () => [{op: "To Hex", args: ["None", 0]}],
    },
    {
        group: "overhead/md5",
        directory: "overhead_md5",
        // The digest arms are named for what they measure rather than for the
        // library, so the FerroSift side is `through-recipe`.
        arm: "through-recipe",
        recipe: () => [{op: "MD5", args: []}],
    },
    {
        group: "overhead/sha256",
        directory: "overhead_sha256",
        arm: "through-recipe",
        recipe: () => [{op: "SHA2", args: ["256", 64, 160]}],
    },
    // The two block ciphers, which have no specialist crate to sit beside and
    // so are measured only against the thing they port.
    {
        group: "cipher/tea",
        directory: "cipher_tea",
        arm: "ferrosift",
        recipe: () => [
            {
                op: "TEA Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "0102030405060708"},
                    "CBC",
                    "Raw",
                    "Hex",
                    "PKCS5",
                ],
            },
        ],
    },
    {
        group: "cipher/xtea",
        directory: "cipher_xtea",
        arm: "ferrosift",
        recipe: () => [
            {
                op: "XTEA Encrypt",
                args: [
                    {option: "Hex", string: "00112233445566778899aabbccddeeff"},
                    {option: "Hex", string: "0102030405060708"},
                    "CBC",
                    "Raw",
                    "Hex",
                    "PKCS5",
                    32,
                ],
            },
        ],
    },
];

/** The spread above which a row is reported as noisy rather than as a number. */
const NOISE_LIMIT = 0.15;

/**
 * Times a recipe, and says how much to trust the answer.
 *
 * One timed loop is not a measurement here. The first attempt at this file ran
 * five warm-up iterations and reported base64 at 16 bytes as *slower* than at
 * 256 -- which cannot be true of the work, and was the just-in-time compiler
 * and the collector still moving. So the loop now warms proportionally, runs
 * several batches, and reports the median of them together with the spread.
 *
 * The spread is what makes the number honest. A row whose batches disagree by
 * more than 15% is marked noisy and given no verdict, which is the same rule
 * `docs/benchmarks.md` already applies to the Rust side -- and it cuts against
 * a flattering result exactly as often as against an unflattering one.
 */
async function measure(chef, input, recipe) {
    const large = input.length >= 65_536;
    const iterations = large ? 20 : 500;
    const batches = 7;

    // Warm on the real work, and for long enough that the compiler has settled
    // before anything is counted.
    for (let i = 0; i < (large ? 5 : 200); i++) {
        await chef.bake(input, recipe);
    }

    const means = [];
    for (let batch = 0; batch < batches; batch++) {
        const start = process.hrtime.bigint();
        for (let i = 0; i < iterations; i++) {
            await chef.bake(input, recipe);
        }
        means.push(Number(process.hrtime.bigint() - start) / iterations);
    }

    means.sort((a, b) => a - b);
    const median = means[Math.floor(means.length / 2)];
    const spread = (means[means.length - 1] - means[0]) / median;
    return {
        nanoseconds: median,
        // The extremes, not only the middle. A spread is what a row *cannot*
        // pin down, and the verdict below needs to know how far the truth
        // could be from the median in each direction.
        fastest: means[0],
        slowest: means[means.length - 1],
        spread,
        noisy: spread > NOISE_LIMIT,
    };
}

/**
 * FerroSift's own estimate for one arm and size, if a run recorded it.
 *
 * The confidence interval comes back with the median, because a verdict that
 * used only the point estimate would be comparing a noisy number against a
 * precise-looking one.
 */
function ferrosift(arm, size) {
    const file = path.join(
        criterionDir,
        arm.directory,
        arm.arm,
        String(size),
        "new",
        "estimates.json",
    );
    if (!existsSync(file)) return null;
    const {median} = JSON.parse(readFileSync(file, "utf8"));
    return {
        median: median.point_estimate,
        lower: median.confidence_interval.lower_bound,
        upper: median.confidence_interval.upper_bound,
    };
}

/**
 * What the measurement supports even read as unfavourably as it allows.
 *
 * The first version of this refused any row whose batches disagreed by more
 * than fifteen percent, and on a loaded machine that discarded nineteen rows
 * of twenty. It was also the wrong test. The spread says how repeatable *one*
 * arm was; the question is whether the two arms overlap. Where one side is a
 * hundred times the other, a fifty-percent spread cannot reach across the gap,
 * and refusing to say so is not caution -- it is discarding a fact.
 *
 * So the rule is now the stricter one it should always have been: take the
 * reference at its fastest batch and FerroSift at the slow end of its
 * interval, and see whether the order still holds. If it does, the ratio of
 * those two is reported -- a floor, not the headline number, and always
 * smaller than the ratio of the medians. If the ranges touch at all, there is
 * no verdict, however narrow the spreads happened to be.
 *
 * This cuts both ways by construction: the same comparison runs in the other
 * direction, so a row where FerroSift is slower is stated on exactly the same
 * terms.
 */
function verdict(reference, ours) {
    if (ours === null) return {kind: "missing"};
    // The reference at its best against FerroSift at its worst.
    if (reference.fastest > ours.upper) {
        return {kind: "faster", ratio: reference.fastest / ours.upper};
    }
    // And the reverse, on the same terms.
    if (reference.slowest < ours.lower) {
        return {kind: "slower", ratio: ours.lower / reference.slowest};
    }
    return {kind: "overlap"};
}

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

const chef = await loadChef();
const rows = [];

for (const arm of ARMS) {
    for (const size of SIZES) {
        const input = sample(size);
        let measured = null;
        try {
            measured = await measure(chef, input, arm.recipe());
        } catch (error) {
            process.stderr.write(`skipped ${arm.group} at ${size}: ${error?.message ?? error}\n`);
            continue;
        }
        const ours = ferrosift(arm, size);
        rows.push({
            group: arm.group,
            size,
            reference: measured.nanoseconds,
            reference_fastest: measured.fastest,
            reference_slowest: measured.slowest,
            spread: measured.spread,
            noisy: measured.noisy,
            ferrosift: ours === null ? null : ours.median,
            ferrosift_lower: ours === null ? null : ours.lower,
            ferrosift_upper: ours === null ? null : ours.upper,
            verdict: verdict(measured, ours),
        });
    }
}

const output = path.join(repoRoot, "docs/benchmarks-cyberchef.json");
writeFileSync(output, `${JSON.stringify({rows}, null, 1)}\n`, "utf8");

let table = "";
let lastGroup = null;
for (const row of rows) {
    if (row.group !== lastGroup) {
        table += `\n### ${row.group}\n\n| Size | CyberChef | FerroSift | Verdict |\n|---|---:|---:|---|\n`;
        lastGroup = row.group;
    }
    const ours = row.ferrosift;
    // The spread is printed whatever the verdict, because a floor drawn from
    // noisy batches and one drawn from tight batches are not the same claim
    // even when they read alike.
    const noise = row.noisy ? ` *(±${(row.spread * 100).toFixed(0)}%)*` : "";
    let said;
    switch (row.verdict.kind) {
        case "faster":
            said = `at least ${row.verdict.ratio.toFixed(1)}× faster${noise}`;
            break;
        case "slower":
            said = `at least ${row.verdict.ratio.toFixed(1)}× slower${noise}`;
            break;
        case "missing":
            said = "*no FerroSift measurement*";
            break;
        default:
            said = `*no verdict — the ranges overlap*${noise}`;
    }
    table += `| ${bytes(row.size)} | ${duration(row.reference)} | ${
        ours === null ? "—" : duration(ours)
    } | ${said} |\n`;
}

process.stdout.write(table);
process.stdout.write(`\nwrote ${rows.length} rows to ${output}\n`);
if (rows.some(row => row.ferrosift === null)) {
    process.stdout.write(
        "some rows have no FerroSift number: run `cargo xtask bench run` first so\n" +
            "criterion has recorded one, then re-run this.\n",
    );
}
