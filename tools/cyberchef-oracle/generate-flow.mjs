// Bakes the flow-control fixture through the pinned reference's own interpreter.
//
// Everything else in the corpus goes through `chef.bake`, which refuses these
// operations outright: "flowControl operations like Return are not currently
// allowed in recipes for chef.bake in the Node API", and Label and Comment are
// not exported as functions at all. That refusal is the Node wrapper's, not the
// reference's -- the browser runs a recipe through `Recipe.execute`, and so
// does this file. Same checkout, same pinned commit, same code path.
//
// Which is why these cases exist rather than an exemption. Fork, Merge, Label
// and Comment used to be listed in docs/compatibility/exemptions.json as
// "pinned elsewhere", meaning a hand-written Rust test asserted what a human
// believed the reference did. They are pinned against the reference's actual
// bytes now, and so are Jump, Conditional Jump, Return and Subsection.
//
// The cases are written out rather than sampled. Flow control is about the
// shape of a recipe, not about the value flowing through it: a second random
// input does not exercise a second path, and a jump that fires on "abc" fires
// on everything. What varies here is the recipe.
import {mkdir, writeFile} from "node:fs/promises";
import path from "node:path";

import {fixtureDirFor, loadCore, runRecipe, selectedProfile} from "./reference.mjs";

const profile = selectedProfile();
const core = await loadCore(profile);
const output = path.join(fixtureDirFor(profile), "flow.json");

const text = value => ({kind: "text", value});
const bytes = hex => ({kind: "bytes", hex});

/** One recipe step. `disabled` is omitted unless it is set. */
const op = (name, args = [], extra = {}) => ({op: name, args, ...extra});

const UPPER = op("To Upper case", ["All"]);
const HEX = op("To Hex", ["None", 0]);
const MERGE = op("Merge", [true]);
const MERGE_ONE = op("Merge", [false]);

const cases = [
    // ---- Label and Comment: markers that the executor must leave alone. ----
    {
        name: "comment_and_label_pass_through",
        input: text("abc"),
        recipe: [op("Comment", ["why this recipe exists"]), op("Label", ["top"]), UPPER],
    },
    {
        // A marker between two typed operations. Both declare that they accept
        // and return anything, which is what makes them transparent to the
        // cross-step type check rather than a break in it.
        name: "comment_between_typed_operations",
        input: bytes("00ff41"),
        recipe: [HEX, op("Comment", ["a note in the middle"]), UPPER],
    },
    {
        name: "label_before_a_typed_operation",
        input: text("ab"),
        recipe: [op("Label", ["start"]), HEX, op("Label", ["end"])],
    },

    // ---- Fork and Merge: the region that already existed, now pinned. ----
    {
        name: "fork_maps_each_line",
        input: text("aGVsbG8=\nb29y"),
        recipe: [
            op("Fork", ["\\n", "\\n", false]),
            op("From Base64", ["A-Za-z0-9+/=", true, false]),
            MERGE,
        ],
    },
    {
        name: "fork_without_merge_runs_to_the_end",
        input: text("ab\ncd"),
        recipe: [op("Fork", ["\\n", "-", false]), UPPER],
    },

    // ---- Jump: the counter moving somewhere other than forward by one. ----
    {
        name: "jump_forward_skips_a_step",
        input: text("abc"),
        recipe: [op("Jump", ["end", 10]), UPPER, op("Label", ["end"]), HEX],
    },
    {
        name: "jump_backward_loops_to_the_limit",
        input: text("aaa"),
        recipe: [op("Label", ["top"]), HEX, op("Jump", ["top", 2])],
    },
    {
        name: "jump_to_a_missing_label_continues",
        input: text("abc"),
        recipe: [op("Jump", ["nowhere", 10]), UPPER],
    },
    {
        name: "jump_with_no_allowance_continues",
        input: text("ab"),
        recipe: [op("Label", ["top"]), HEX, op("Jump", ["top", 0])],
    },
    {
        // The reference's label search does not ask whether the step is
        // enabled, so a disabled Label is still a destination.
        name: "jump_to_a_disabled_label",
        input: text("ab"),
        recipe: [op("Label", ["top"], {disabled: true}), HEX, op("Jump", ["top", 1])],
    },
    {
        name: "jump_inside_a_fork_is_scoped_to_the_branch",
        input: text("ab\ncd"),
        recipe: [
            op("Fork", ["\\n", "\\n", false]),
            op("Label", ["top"]),
            HEX,
            op("Jump", ["top", 1]),
            MERGE,
        ],
    },
    {
        // A branch is its own recipe there, so a label outside it is not
        // visible from inside and the jump simply does not fire.
        name: "jump_out_of_a_fork_does_not_fire",
        input: text("ab\ncd"),
        recipe: [
            op("Label", ["outer"]),
            op("Fork", ["\\n", "\\n", false]),
            op("Jump", ["outer", 5]),
            MERGE,
            HEX,
        ],
    },
    {
        // Two jump sites sharing one allowance, which is what the reference's
        // single `numJumps` counter means.
        name: "two_jumps_share_one_allowance",
        input: text("ab"),
        recipe: [
            op("Label", ["top"]),
            HEX,
            op("Jump", ["top", 2]),
            op("Label", ["second"]),
            HEX,
            op("Jump", ["second", 2]),
        ],
    },

    // ---- Conditional Jump: the same, decided by a pattern. ----
    {
        name: "conditional_jump_taken",
        input: text("abc"),
        recipe: [op("Label", ["top"]), HEX, op("Conditional Jump", ["^61", false, "top", 2])],
    },
    {
        name: "conditional_jump_not_taken",
        input: text("abc"),
        recipe: [op("Conditional Jump", ["zzz", false, "end", 10]), UPPER, op("Label", ["end"])],
    },
    {
        name: "conditional_jump_inverted",
        input: text("abc"),
        recipe: [op("Conditional Jump", ["zzz", true, "end", 10]), UPPER, op("Label", ["end"])],
    },
    {
        // An empty pattern tests nothing, and the reference never reaches the
        // branch that would clear its counter.
        name: "conditional_jump_empty_pattern_does_nothing",
        input: text("abc"),
        recipe: [op("Conditional Jump", ["", false, "end", 10]), UPPER, op("Label", ["end"])],
    },
    {
        name: "conditional_jump_missing_label_continues",
        input: text("abc"),
        recipe: [op("Conditional Jump", ["b", false, "nowhere", 10]), UPPER],
    },
    {
        // A loop that ends because the condition stops holding rather than
        // because the allowance ran out.
        name: "conditional_jump_loops_until_the_pattern_fails",
        input: text("aaaab"),
        recipe: [
            op("Label", ["top"]),
            op("Drop bytes", [0, 1, false]),
            op("Conditional Jump", ["a", false, "top", 10]),
        ],
    },
    {
        name: "conditional_jump_case_matters",
        input: text("ABC"),
        recipe: [op("Conditional Jump", ["abc", false, "end", 10]), UPPER, op("Label", ["end"])],
    },

    // ---- Return: the counter stopping. ----
    {
        name: "return_stops_the_recipe",
        input: text("abc"),
        recipe: [UPPER, op("Return"), HEX],
    },
    {
        name: "return_first_leaves_the_input_alone",
        input: text("abc"),
        recipe: [op("Return"), UPPER],
    },
    {
        // A branch is its own recipe, so this returns from the branch.
        name: "return_inside_a_fork_ends_the_branch",
        input: text("ab\ncd"),
        recipe: [op("Fork", ["\\n", "\\n", false]), UPPER, op("Return"), HEX, MERGE],
    },
    {
        name: "return_after_a_jump",
        input: text("ab"),
        recipe: [op("Label", ["top"]), HEX, op("Jump", ["top", 1]), op("Return"), UPPER],
    },

    // ---- Subsection: a region over spans instead of branches. ----
    {
        name: "subsection_uppercases_each_match",
        input: text("aa-bb-cc"),
        recipe: [op("Subsection", ["[a-z]+", true, true, false]), UPPER, MERGE],
    },
    {
        name: "subsection_without_global_takes_the_first",
        input: text("aa-bb-cc"),
        recipe: [op("Subsection", ["[a-z]+", true, false, false]), UPPER, MERGE],
    },
    {
        name: "subsection_case_insensitive",
        input: text("Aa-bB-cc"),
        recipe: [op("Subsection", ["[a-z]+", false, true, false]), HEX, MERGE],
    },
    {
        // With a capture group the region runs on the group, so the delimiter
        // that found it survives untouched.
        name: "subsection_capture_group",
        input: text("key=val;key2=val2"),
        recipe: [op("Subsection", ["=(\\w+)", true, true, false]), UPPER, MERGE],
    },
    {
        name: "subsection_no_match_skips_the_region",
        input: text("123"),
        recipe: [op("Subsection", ["[a-z]+", true, true, false]), UPPER, MERGE, HEX],
    },
    {
        // An empty pattern selects nothing to scope, and the following steps
        // run on the whole value as if the Subsection were not there.
        name: "subsection_empty_pattern_runs_on_everything",
        input: text("abc"),
        recipe: [op("Subsection", ["", true, true, false]), UPPER, MERGE],
    },
    {
        name: "subsection_without_merge_runs_to_the_end",
        input: text("a1b2"),
        recipe: [op("Subsection", ["[a-z]", true, true, false]), UPPER],
    },
    {
        name: "subsection_then_more_operations",
        input: text("a1b2"),
        recipe: [op("Subsection", ["[a-z]", true, true, false]), UPPER, MERGE, HEX],
    },
    {
        name: "subsection_matching_everything",
        input: text("abc"),
        recipe: [op("Subsection", [".+", true, true, false]), HEX, MERGE],
    },
    {
        name: "subsection_over_bytes",
        input: bytes("6100620063"),
        recipe: [op("Subsection", ["[a-z]", true, true, false]), UPPER, MERGE],
    },
    {
        // Nested regions, closed inside-out by a Merge that does not merge all.
        name: "subsection_nested_in_a_fork",
        input: text("ab-cd\nef-gh"),
        recipe: [
            op("Fork", ["\\n", "\\n", false]),
            op("Subsection", ["[a-z]+", true, true, false]),
            UPPER,
            MERGE_ONE,
            MERGE,
        ],
    },
    {
        name: "fork_nested_in_a_subsection",
        input: text("<a,b> and <c,d>"),
        recipe: [
            op("Subsection", ["<([^>]+)>", true, true, false]),
            op("Fork", [",", ";", false]),
            UPPER,
            MERGE_ONE,
            MERGE,
        ],
    },
    {
        name: "subsection_containing_a_jump",
        input: text("a-b"),
        recipe: [
            op("Subsection", ["[a-z]", true, true, false]),
            op("Label", ["top"]),
            HEX,
            op("Jump", ["top", 1]),
            MERGE,
        ],
    },
    {
        name: "subsection_containing_a_return",
        input: text("a-b"),
        recipe: [op("Subsection", ["[a-z]", true, true, false]), HEX, op("Return"), UPPER, MERGE],
    },
    {
        name: "two_subsections_in_a_row",
        input: text("a1-b2"),
        recipe: [
            op("Subsection", ["[a-z]", true, true, false]),
            UPPER,
            MERGE,
            op("Subsection", ["[0-9]", true, true, false]),
            HEX,
            MERGE,
        ],
    },
];

const names = new Set();
for (const one of cases) {
    if (names.has(one.name)) throw new Error(`duplicate case name ${one.name}`);
    names.add(one.name);
}

let failures = 0;
for (const testCase of cases) {
    testCase.outputs_hex = [];
    for (let length = 1; length <= testCase.recipe.length; length += 1) {
        try {
            const {hex, progress} = await runRecipe(
                core,
                testCase.input,
                testCase.recipe.slice(0, length),
            );
            if (progress !== length) {
                // An operation failed part-way, so the dish holds an error
                // message rather than an answer. The other generators drop
                // those and so does this one -- a pinned error string is a
                // pinned message, not pinned behaviour.
                failures += 1;
                process.stderr.write(
                    `incomplete: ${testCase.name} prefix ${length} stopped after ${progress}\n`,
                );
                break;
            }
            testCase.outputs_hex.push(hex);
        } catch (error) {
            failures += 1;
            process.stderr.write(
                `bake failed: ${testCase.name} prefix ${length}: ${error?.message ?? error}\n`,
            );
            break;
        }
    }
    testCase.stopped_after = testCase.outputs_hex.length;
}

const complete = cases.filter(testCase => testCase.stopped_after === testCase.recipe.length);

const suite = {
    reference: {name: "CyberChef", version: profile.version, commit: profile.commit},
    cases: complete,
};

await mkdir(path.dirname(output), {recursive: true});
await writeFile(output, `${JSON.stringify(suite, null, 1)}\n`, "utf8");
process.stdout.write(
    `wrote ${complete.length} flow cases (${failures} dropped) to ${output}\n`,
);
