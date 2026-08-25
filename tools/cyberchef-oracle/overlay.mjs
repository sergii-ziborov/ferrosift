// Condenses a non-baseline profile's fixtures into a delta against the baseline.
//
// Two profiles that agree everywhere produce two identical million-byte files,
// and a third would produce a third. That is the wrong shape: the interesting
// content of a second profile is exactly where it *differs*, and storing the
// agreement again says nothing a diff could not.
//
// So a non-baseline profile is stored as an overlay: the cases whose output
// changed, plus the cases added and removed. `tests/profiles.rs` reconstructs
// the full profile from baseline + overlay and replays FerroSift against the
// result, so the evidence stays direct — FerroSift is checked against 11.4's
// actual bytes, not against an argument that 11.4 equals 11.3.
//
// An empty overlay is a real finding, not a missing file: it is the record
// that upstream changed nothing this corpus can see.
import {mkdir, readFile, writeFile} from "node:fs/promises";
import path from "node:path";

import {DEFAULT_PROFILE, PROFILES, fixtureDirFor, selectedProfile} from "./reference.mjs";

const profile = selectedProfile();
const baseline = PROFILES[DEFAULT_PROFILE];

if (profile.version === baseline.version) {
    process.stderr.write(
        `${profile.version} is the baseline profile; it is stored in full, not as an overlay\n`,
    );
    process.exit(1);
}

/** Reads one generated fixture file. */
async function load(forProfile, name) {
    const file = path.join(fixtureDirFor(forProfile), `${name}.json`);
    try {
        return JSON.parse(await readFile(file, "utf8"));
    } catch (error) {
        if (error.code === "ENOENT") {
            throw new Error(
                `missing ${file}\nrun: cargo xtask cyberchef generate --profile ${forProfile.version}`,
            );
        }
        throw error;
    }
}

/**
 * Computes the delta between two generated suites.
 *
 * Cases are matched by name because that is what the generator guarantees is
 * stable; ordering is not, and matching by index would report a reordering as
 * a behavioural change.
 */
function delta(base, compared) {
    const before = new Map(base.cases.map(one => [one.name, one]));
    const after = new Map(compared.cases.map(one => [one.name, one]));

    const changed = [];
    for (const [name, one] of after) {
        const was = before.get(name);
        if (!was) continue;
        if (JSON.stringify(was.outputs_hex) !== JSON.stringify(one.outputs_hex)) {
            changed.push(one);
        }
    }

    return {
        changed,
        added: [...after.values()].filter(one => !before.has(one.name)),
        removed: [...before.keys()].filter(name => !after.has(name)).sort(),
    };
}

const written = [];
for (const name of ["corpus", "differential"]) {
    const base = await load(baseline, name);
    const compared = await load(profile, name);
    const {changed, added, removed} = delta(base, compared);

    written.push({
        name,
        overlay: {
            reference: {name: "CyberChef", version: profile.version, commit: profile.commit},
            baseline: {version: baseline.version, commit: baseline.commit},
            // Recorded so a reader knows how much agreement the empty lists
            // below stand for. An overlay with no changes over four cases
            // would mean far less than one over two thousand.
            compared_cases: compared.cases.length,
            changed,
            added,
            removed,
        },
    });
}

const directory = fixtureDirFor(profile);
await mkdir(directory, {recursive: true});
for (const {name, overlay} of written) {
    const file = path.join(directory, `${name}.overlay.json`);
    await writeFile(file, `${JSON.stringify(overlay, null, 1)}\n`, "utf8");
    process.stdout.write(
        `${name}: ${overlay.compared_cases} cases, ` +
            `${overlay.changed.length} changed, ${overlay.added.length} added, ` +
            `${overlay.removed.length} removed -> ${file}\n`,
    );
}
