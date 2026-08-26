// Differential corpus for the pattern language.
//
// The catalog side of FerroSift is pinned against a real CyberChef checkout;
// the pattern side had nothing. Sixty-six tests, every one of them asking the
// crate what the crate does. That is not evidence of compatibility with
// anything, and the subset page said so.
//
// This runs the same source and the same bytes through ImHex's own `plcli`,
// built from a pinned checkout, and records what it answered. A replay test
// then asks FerroSift for the same thing. Where they differ, the difference is
// a fact rather than an opinion.
//
// The reference renders with its JSON formatter, so the comparison is against
// a shape it chose: a struct is an object, an array is an array, an enum is
// `"Name::Constant"`, a bitfield is an object of its members, a `char` is a
// one-character string. Rendering FerroSift's node tree into that shape is
// what the replay does -- reaching past either side to compare internals would
// compare two designs rather than two answers.

import {execFileSync} from "node:child_process";
import {mkdirSync, readFileSync, writeFileSync, existsSync} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import os from "node:os";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../..");
const checkout = path.join(here, "vendor/PatternLanguage");
const cli = path.join(checkout, "build/cli/plcli.exe");
const fallback = path.join(checkout, "build/cli/plcli");

/** The pinned reference binary, or an explanation of how to get one. */
function reference() {
    for (const candidate of [cli, fallback]) {
        if (existsSync(candidate)) return candidate;
    }
    throw new Error(
        `no pattern-language reference at ${cli}\n` +
            "build it with:\n" +
            "    cargo xtask pattern setup",
    );
}

/**
 * Every case: a name, the pattern source, and the bytes it reads.
 *
 * Chosen to separate the constructs rather than to exercise them together. A
 * case that used four features at once would report a single failure for
 * whichever of them was wrong first.
 */
const CASES = [
    {
        name: "scalar_widths",
        pattern: `struct S {
    u8 a;
    u16 b;
    u32 c;
    u64 d;
};
S s @ 0x00;`,
        data: "01" + "0203" + "04050607" + "08090a0b0c0d0e0f",
    },
    {
        name: "endianness_prefix",
        pattern: `struct S {
    be u16 big;
    le u16 little;
};
S s @ 0x00;`,
        data: "cafebabe",
    },
    {
        name: "signed_widths",
        pattern: `struct S {
    s8 a;
    s16 b;
    s32 c;
};
S s @ 0x00;`,
        data: "ff" + "fffe" + "fffffffd",
    },
    {
        name: "fixed_array",
        pattern: `struct S {
    u8 items[4];
};
S s @ 0x00;`,
        data: "11223344",
    },
    {
        name: "enum_named_and_unnamed",
        pattern: `enum Kind : u8 { None = 0, Alpha = 1, Beta = 2 };
struct S {
    Kind known;
    Kind unknown;
};
S s @ 0x00;`,
        data: "02" + "7f",
    },
    {
        // The layout question: which end of the byte the first member reads
        // from. Two members of unequal width make the two answers different
        // numbers rather than the same one.
        name: "bitfield_member_order",
        pattern: `bitfield Flags {
    low  : 3;
    high : 5;
};
Flags flags @ 0x00;`,
        data: "a5",
    },
    {
        name: "bitfield_across_two_bytes",
        pattern: `bitfield Wide {
    a : 4;
    b : 8;
    c : 4;
};
Wide wide @ 0x00;`,
        data: "1234",
    },
    {
        name: "char_and_bool",
        pattern: `struct S {
    char letter;
    bool yes;
    bool no;
};
S s @ 0x00;`,
        data: "41" + "01" + "00",
    },
    {
        name: "nested_struct",
        pattern: `struct Inner { u8 x; u8 y; };
struct Outer { Inner first; Inner second; };
Outer outer @ 0x00;`,
        data: "01020304",
    },
    {
        name: "placement_offset",
        pattern: `struct S { u8 v; };
S first  @ 0x00;
S second @ 0x03;`,
        data: "aabbccdd",
    },
    {
        name: "using_alias",
        pattern: `using Word = be u16;
struct S { Word a; Word b; };
S s @ 0x00;`,
        data: "0102" + "0304",
    },
    {
        name: "float_and_double",
        pattern: `struct S {
    float f;
    double d;
};
S s @ 0x00;`,
        data: "0000803f" + "000000000000f03f",
    },
];

const outputDir = path.join(repoRoot, "crates/ferrosift-pattern/tests/fixtures");
mkdirSync(outputDir, {recursive: true});

const binary = reference();
const commit = execFileSync("git", ["-C", checkout, "rev-parse", "HEAD"], {
    encoding: "utf8",
}).trim();

const temporary = path.join(os.tmpdir(), "ferrosift-pattern-oracle");
mkdirSync(temporary, {recursive: true});

const recorded = [];
let failures = 0;
for (const testCase of CASES) {
    const patternFile = path.join(temporary, `${testCase.name}.hexpat`);
    const dataFile = path.join(temporary, `${testCase.name}.bin`);
    const outputFile = path.join(temporary, `${testCase.name}.json`);
    writeFileSync(patternFile, `${testCase.pattern}\n`, "utf8");
    writeFileSync(dataFile, Buffer.from(testCase.data, "hex"));

    try {
        execFileSync(
            binary,
            ["format", "-p", patternFile, "-i", dataFile, "-f", "json", "-o", outputFile],
            {stdio: "pipe"},
        );
    } catch (error) {
        failures += 1;
        process.stderr.write(
            `reference refused ${testCase.name}: ${error?.stderr?.toString() ?? error}\n`,
        );
        continue;
    }

    recorded.push({
        name: testCase.name,
        pattern: testCase.pattern,
        data: testCase.data,
        // Stored as the reference wrote it, whitespace included: the formatter
        // chose that shape and a reformat here would compare a choice of mine.
        json: readFileSync(outputFile, "utf8"),
    });
}

const fixture = path.join(outputDir, "imhex.json");
writeFileSync(
    fixture,
    `${JSON.stringify(
        {reference: {name: "ImHex PatternLanguage", commit}, cases: recorded},
        null,
        1,
    )}\n`,
    "utf8",
);
process.stdout.write(
    `wrote ${recorded.length} pattern cases (${failures} refused) to ${fixture}\n`,
);
