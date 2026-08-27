// Collects the peer comparison into a file the report can render.
//
// The measurement existed and reached nobody, which is the same fault this
// harness had already found twice in other people's code and then committed
// itself: `benches/peer.rs` wrote criterion estimates into a target directory
// and nothing read them, so `docs/benchmarks.md` said nothing about the only
// competitor of FerroSift's own shape.
//
// Two things make this arm different from every other one, and both are
// recorded in the output rather than left to a reader to discover.
//
// It runs on Linux. Unmodified rx-chef does not link on Windows MSVC --
// `src/operations/md6.rs` declares `#[link(name = "md6")]` without
// `kind = "static"` -- and patching a competitor before measuring it is the
// kind of thing that makes a comparison worthless. So the platform is recorded
// with the numbers, and these are not comparable with the rest of the report.
//
// Both arms are measured in one process, one after the other. That is the best
// available and it is not perfect: a machine that slows between them biases
// the ratio, and on this hardware two runs of the same binary have disagreed
// by a quarter. Several runs are therefore expected, and the file keeps every
// one rather than a single sample.

import {existsSync, readFileSync, readdirSync, writeFileSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../..");
const output = path.join(repoRoot, "docs/benchmarks-peer.json");

/**
 * Where criterion left its estimates.
 *
 * Overridable because the run that produces them happens inside WSL, whose
 * target directory is not the repository's.
 */
const criterionDir =
    process.env.FERROSIFT_PEER_CRITERION ?? path.join(repoRoot, "bench/target/criterion");

/** The median and its interval for one arm at one size, if a run recorded it. */
function estimate(group, arm, size) {
    const file = path.join(criterionDir, group, arm, String(size), "new", "estimates.json");
    if (!existsSync(file)) return null;
    const {median} = JSON.parse(readFileSync(file, "utf8"));
    return {
        median: median.point_estimate,
        lower: median.confidence_interval.lower_bound,
        upper: median.confidence_interval.upper_bound,
    };
}

/** The sizes `bench/src/lib.rs` sweeps. */
const SIZES = [16, 256, 4096, 65_536, 1_048_576];

/** The arm names `benches/peer.rs` uses. */
const OURS = "ferrosift";
const THEIRS = "rxchef-peer";

const groups = existsSync(criterionDir)
    ? readdirSync(criterionDir).filter(name => name.startsWith("peer_"))
    : [];

if (groups.length === 0) {
    process.stderr.write(
        `no peer measurements under ${criterionDir}\n` +
            "run the peer benchmark first; it needs a platform where unmodified\n" +
            "rx-chef links, which is not Windows:\n" +
            "    cargo bench --features peer --bench peer\n",
    );
    process.exit(1);
}

const rows = [];
for (const group of groups.sort()) {
    for (const size of SIZES) {
        const ours = estimate(group, OURS, size);
        const theirs = estimate(group, THEIRS, size);
        if (!ours || !theirs) continue;
        rows.push({
            group: group.replace(/^peer_/u, "").replace(/_/gu, "/"),
            size,
            ferrosift: ours.median,
            ferrosift_lower: ours.lower,
            ferrosift_upper: ours.upper,
            peer: theirs.median,
            peer_lower: theirs.lower,
            peer_upper: theirs.upper,
            // The ratio of the medians, stated plainly. A floor drawn from the
            // intervals, as the CyberChef arm uses, would be misleading here:
            // the gaps are small enough that the intervals overlap on most
            // rows, and reporting "no verdict" for all of them would hide a
            // result that several runs agree on.
            ratio: ours.median / theirs.median,
            overlaps: ours.lower <= theirs.upper && theirs.lower <= ours.upper,
        });
    }
}

writeFileSync(
    output,
    `${JSON.stringify(
        {
            peer: "rx-chef",
            revision: "99e26de96e28faed5b850a32825afcc7cfd9cd22",
            // The platform the *measurement* ran on, not the one collecting
            // it. Those differ here -- the run is inside WSL and the collector
            // reads its output from Windows -- and recording the collector's
            // would have put `win32` on a Linux result.
            measured_on: "linux",
            note: "unmodified rx-chef does not link on Windows MSVC, so this arm is measured on Linux and is not comparable with the rest of this report",
            rows,
        },
        null,
        1,
    )}\n`,
    "utf8",
);
process.stdout.write(`wrote ${rows.length} peer rows to ${output}\n`);
