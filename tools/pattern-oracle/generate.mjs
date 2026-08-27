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
import {mkdirSync, readFileSync, rmSync, writeFileSync, existsSync} from "node:fs";
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

    // --- bit layout -------------------------------------------------------
    // Which end of the span a member reads from, and which end of the span is
    // which byte. Every case here has members of unequal width or crosses a
    // byte boundary, because a bitfield of equal members inside one byte gives
    // the same answer under either convention and proves nothing.
    {
        name: "bitfield_single_bits",
        pattern: `bitfield Bits {
    a : 1;
    b : 1;
    c : 1;
    d : 1;
    e : 1;
    f : 1;
    g : 1;
    h : 1;
};
Bits bits @ 0x00;`,
        data: "b3",
    },
    {
        name: "bitfield_uneven_three_members",
        pattern: `bitfield Split {
    first  : 2;
    second : 3;
    third  : 3;
};
Split split @ 0x00;`,
        data: "9c",
    },
    {
        name: "bitfield_spanning_three_bytes",
        pattern: `bitfield Long {
    a : 5;
    b : 11;
    c : 8;
};
Long long @ 0x00;`,
        data: "0f1e2d",
    },
    {
        name: "bitfield_partial_byte",
        pattern: `bitfield Partial {
    a : 3;
    b : 2;
};
Partial partial @ 0x00;`,
        data: "ff",
    },
    {
        name: "bitfield_thirty_two_bits",
        pattern: `bitfield Word {
    low  : 16;
    high : 16;
};
Word word @ 0x00;`,
        data: "0102" + "0304",
    },
    {
        name: "bitfield_be_prefix",
        pattern: `bitfield Flags {
    a : 4;
    b : 12;
};
struct S {
    be Flags flags;
};
S s @ 0x00;`,
        data: "abcd",
    },
    {
        name: "bitfield_le_prefix",
        pattern: `bitfield Flags {
    a : 4;
    b : 12;
};
struct S {
    le Flags flags;
};
S s @ 0x00;`,
        data: "abcd",
    },
    {
        name: "bitfield_inside_struct_after_scalar",
        pattern: `bitfield Flags {
    a : 4;
    b : 4;
};
struct S {
    u16 lead;
    Flags flags;
    u8 trail;
};
S s @ 0x00;`,
        data: "1122" + "3c" + "44",
    },
    {
        name: "bitfield_array",
        pattern: `bitfield Pair {
    a : 3;
    b : 5;
};
struct S {
    Pair items[3];
};
S s @ 0x00;`,
        data: "a5" + "5a" + "0f",
    },

    // --- widths and signs -------------------------------------------------
    {
        name: "signed_negative_and_positive",
        pattern: `struct S {
    s8 neg;
    s8 pos;
    s16 wide_neg;
    s16 wide_pos;
    s64 huge;
};
S s @ 0x00;`,
        data: "80" + "7f" + "8000" + "7fff" + "8000000000000000",
    },
    {
        name: "unsigned_extremes",
        pattern: `struct S {
    u8 a;
    u16 b;
    u32 c;
    u64 d;
};
S s @ 0x00;`,
        data: "ff" + "ffff" + "ffffffff" + "ffffffffffffffff",
    },
    {
        name: "big_endian_all_widths",
        pattern: `struct S {
    be u16 b;
    be u32 c;
    be u64 d;
};
S s @ 0x00;`,
        data: "0102" + "03040506" + "0708090a0b0c0d0e",
    },
    {
        name: "little_endian_all_widths",
        pattern: `struct S {
    le u16 b;
    le u32 c;
    le u64 d;
};
S s @ 0x00;`,
        data: "0102" + "03040506" + "0708090a0b0c0d0e",
    },
    {
        name: "endianness_switches_mid_struct",
        pattern: `struct S {
    be u32 first;
    le u32 second;
    be u32 third;
};
S s @ 0x00;`,
        data: "00000001" + "00000001" + "00000001",
    },
    {
        name: "signed_endianness",
        pattern: `struct S {
    be s16 big;
    le s16 little;
};
S s @ 0x00;`,
        data: "ff01" + "ff01",
    },
    {
        name: "float_negative_and_fractional",
        pattern: `struct S {
    float half;
    float negative;
    double third;
};
S s @ 0x00;`,
        data: "0000003f" + "000080bf" + "555555555555d53f",
    },
    {
        name: "float_endianness",
        pattern: `struct S {
    be float big;
    le float little;
};
S s @ 0x00;`,
        data: "3f800000" + "0000803f",
    },

    // --- arrays -----------------------------------------------------------
    {
        name: "array_of_structs",
        pattern: `struct Item { u8 tag; u16 value; };
struct S { Item items[3]; };
S s @ 0x00;`,
        data: "01" + "0203" + "04" + "0506" + "07" + "0809",
    },
    {
        name: "array_of_arrays",
        pattern: `struct Row { u8 cells[3]; };
struct S { Row rows[2]; };
S s @ 0x00;`,
        data: "010203" + "040506",
    },
    {
        name: "array_length_from_expression",
        pattern: `struct S {
    u8 count;
    u8 items[count];
};
S s @ 0x00;`,
        data: "03" + "0a0b0c",
    },
    {
        name: "array_length_from_arithmetic",
        pattern: `struct S {
    u8 count;
    u8 items[count * 2 + 1];
};
S s @ 0x00;`,
        data: "02" + "0102030405",
    },
    {
        name: "array_length_zero",
        pattern: `struct S {
    u8 count;
    u8 items[count];
    u8 after;
};
S s @ 0x00;`,
        data: "00" + "ff",
    },
    {
        name: "array_of_enums",
        pattern: `enum Kind : u8 { A = 1, B = 2 };
struct S { Kind kinds[3]; };
S s @ 0x00;`,
        data: "01" + "02" + "09",
    },
    {
        name: "char_array",
        pattern: `struct S { char text[5]; };
S s @ 0x00;`,
        data: "48656c6c6f",
    },
    {
        name: "while_array_bounded_by_offset",
        pattern: `struct S {
    u8 items[while($ < 3)];
    u8 last;
};
S s @ 0x00;`,
        data: "01020304",
    },

    // --- enums ------------------------------------------------------------
    {
        name: "enum_implicit_values",
        pattern: `enum Kind : u8 { Zero, One, Two, Three };
struct S { Kind a; Kind b; Kind c; };
S s @ 0x00;`,
        data: "00" + "02" + "03",
    },
    {
        name: "enum_wide_backing",
        pattern: `enum Wide : u32 { Small = 1, Large = 0x11223344 };
struct S { Wide a; Wide b; };
S s @ 0x00;`,
        data: "01000000" + "44332211",
    },
    {
        name: "enum_big_endian_at_use_site",
        pattern: `enum Wide : u16 { One = 0x0102 };
struct S { be Wide a; le Wide b; };
S s @ 0x00;`,
        data: "0102" + "0201",
    },
    {
        name: "enum_signed_backing",
        pattern: `enum Sign : s8 { Minus = -1, Zero = 0 };
struct S { Sign a; Sign b; };
S s @ 0x00;`,
        data: "ff" + "00",
    },
    {
        name: "enum_expression_values",
        pattern: `enum Kind : u8 { A = 1 << 2, B = (1 << 2) + 1 };
struct S { Kind a; Kind b; };
S s @ 0x00;`,
        data: "04" + "05",
    },

    // --- composites -------------------------------------------------------
    {
        name: "union_overlays_members",
        pattern: `union U {
    u32 whole;
    u8 bytes[4];
};
U u @ 0x00;`,
        data: "01020304",
    },
    {
        name: "union_of_unequal_widths",
        pattern: `union U {
    u8 small;
    u64 large;
};
struct S { U u; u8 after; };
S s @ 0x00;`,
        data: "0102030405060708" + "ff",
    },
    {
        name: "nested_struct_three_deep",
        pattern: `struct A { u8 x; };
struct B { A a; u8 y; };
struct C { B b; u8 z; };
C c @ 0x00;`,
        data: "010203",
    },
    {
        name: "struct_containing_union",
        pattern: `union U { u16 word; u8 pair[2]; };
struct S { u8 lead; U body; u8 tail; };
S s @ 0x00;`,
        data: "aa" + "0102" + "bb",
    },
    {
        name: "empty_struct",
        pattern: `struct Empty {};
struct S { Empty e; u8 after; };
S s @ 0x00;`,
        data: "7f",
    },

    // --- padding and placement --------------------------------------------
    {
        name: "padding_skips_bytes",
        pattern: `struct S {
    u8 first;
    padding[3];
    u8 last;
};
S s @ 0x00;`,
        data: "01" + "aabbcc" + "02",
    },
    {
        name: "padding_zero",
        pattern: `struct S {
    u8 first;
    padding[0];
    u8 second;
};
S s @ 0x00;`,
        data: "0102",
    },
    {
        name: "placement_overlapping",
        pattern: `struct S { u16 v; };
S first  @ 0x00;
S second @ 0x01;`,
        data: "01020304",
    },
    {
        name: "placement_at_expression",
        pattern: `struct S { u8 v; };
S here @ 1 + 2;`,
        data: "00000004",
    },
    {
        name: "multiple_top_level_scalars",
        pattern: `u8 a @ 0x00;
u16 b @ 0x01;
u32 c @ 0x03;`,
        data: "01" + "0203" + "04050607",
    },

    // --- expressions ------------------------------------------------------
    {
        name: "expression_precedence",
        pattern: `struct S {
    u8 items[1 + 2 * 3];
};
S s @ 0x00;`,
        data: "0102030405060708",
    },
    {
        name: "expression_shifts_and_masks",
        pattern: `struct S {
    u8 items[(1 << 3) - (6 & 3)];
};
S s @ 0x00;`,
        data: "0102030405060708",
    },
    {
        name: "expression_from_earlier_member",
        pattern: `struct S {
    u8 a;
    u8 b;
    u8 items[a + b];
};
S s @ 0x00;`,
        data: "01" + "02" + "0a0b0c",
    },
    {
        name: "sizeof_type",
        pattern: `struct Inner { u32 a; u16 b; };
struct S {
    u8 items[sizeof(Inner)];
};
S s @ 0x00;`,
        data: "0102030405060708",
    },
    {
        name: "if_selects_a_member",
        pattern: `struct S {
    u8 tag;
    if (tag == 1) {
        u16 pair;
    } else {
        u8 single;
    }
};
S s @ 0x00;`,
        data: "01" + "0203",
    },
    {
        name: "if_else_takes_the_other_branch",
        pattern: `struct S {
    u8 tag;
    if (tag == 1) {
        u16 pair;
    } else {
        u8 single;
    }
};
S s @ 0x00;`,
        data: "02" + "03",
    },

    // --- aliases ----------------------------------------------------------
    {
        name: "using_alias_of_struct",
        pattern: `struct Inner { u8 a; u8 b; };
using Pair = Inner;
struct S { Pair one; Pair two; };
S s @ 0x00;`,
        data: "01020304",
    },
    {
        name: "using_alias_of_enum",
        pattern: `enum Kind : u8 { A = 1 };
using K = Kind;
struct S { K value; };
S s @ 0x00;`,
        data: "01",
    },
    {
        name: "using_alias_chain",
        pattern: `using A = be u32;
using B = A;
struct S { B value; };
S s @ 0x00;`,
        data: "01020304",
    },

    // --- more bit layout --------------------------------------------------
    {
        name: "bitfield_one_bit_then_seven",
        pattern: `bitfield Flag {
    set  : 1;
    rest : 7;
};
Flag flag @ 0x00;`,
        data: "81",
    },
    {
        name: "bitfield_seven_then_one",
        pattern: `bitfield Flag {
    rest : 7;
    set  : 1;
};
Flag flag @ 0x00;`,
        data: "81",
    },
    {
        name: "bitfield_crossing_every_byte_boundary",
        pattern: `bitfield Crossing {
    a : 7;
    b : 7;
    c : 7;
    d : 3;
};
Crossing crossing @ 0x00;`,
        data: "deadbeef",
    },
    {
        name: "bitfield_wider_than_a_word",
        pattern: `bitfield Big {
    a : 20;
    b : 20;
    c : 24;
};
Big big @ 0x00;`,
        data: "0123456789abcdef",
    },
    {
        name: "bitfield_all_zero_bytes",
        pattern: `bitfield Flags {
    a : 3;
    b : 5;
};
Flags flags @ 0x00;`,
        data: "00",
    },
    {
        name: "bitfield_all_one_bytes",
        pattern: `bitfield Flags {
    a : 3;
    b : 13;
};
Flags flags @ 0x00;`,
        data: "ffff",
    },
    {
        name: "two_bitfields_in_sequence",
        pattern: `bitfield A { x : 4; y : 4; };
bitfield B { p : 2; q : 6; };
struct S { A first; B second; };
S s @ 0x00;`,
        data: "12" + "34",
    },
    {
        name: "bitfield_at_nonzero_placement",
        pattern: `bitfield Flags { a : 3; b : 5; };
Flags flags @ 0x02;`,
        data: "0000a5ff",
    },

    // --- offsets and cursors ----------------------------------------------
    {
        name: "dollar_offset_in_array_length",
        pattern: `struct S {
    u8 lead;
    u8 items[$];
};
S s @ 0x00;`,
        data: "01" + "0203",
    },
    {
        name: "placement_at_end_of_data",
        pattern: `struct S { u8 v; };
S last @ 0x03;`,
        data: "00000041",
    },
    {
        name: "two_placements_descending",
        pattern: `struct S { u8 v; };
S high @ 0x02;
S low  @ 0x00;`,
        data: "0a0b0c",
    },
    {
        name: "padding_then_array",
        pattern: `struct S {
    padding[2];
    u8 items[2];
};
S s @ 0x00;`,
        data: "ffff" + "0102",
    },
    {
        name: "padding_inside_nested_struct",
        pattern: `struct Inner {
    u8 a;
    padding[1];
    u8 b;
};
struct S { Inner one; Inner two; };
S s @ 0x00;`,
        data: "01ff02" + "03ff04",
    },

    // --- conditionals -----------------------------------------------------
    {
        name: "if_without_else_taken",
        pattern: `struct S {
    u8 tag;
    if (tag > 0) {
        u8 extra;
    }
    u8 tail;
};
S s @ 0x00;`,
        data: "01" + "02" + "03",
    },
    {
        name: "if_without_else_skipped",
        pattern: `struct S {
    u8 tag;
    if (tag > 0) {
        u8 extra;
    }
    u8 tail;
};
S s @ 0x00;`,
        data: "00" + "03",
    },
    {
        name: "nested_if",
        pattern: `struct S {
    u8 outer;
    if (outer == 1) {
        u8 inner;
        if (inner == 2) {
            u8 deep;
        }
    }
};
S s @ 0x00;`,
        data: "01" + "02" + "03",
    },
    {
        name: "if_on_comparison_operators",
        pattern: `struct S {
    u8 v;
    if (v >= 5 && v <= 10) { u8 inside; }
    if (v < 5 || v > 10)   { u8 outside; }
};
S s @ 0x00;`,
        data: "07" + "aa",
    },
    {
        name: "if_on_equality_with_enum",
        pattern: `enum Kind : u8 { A = 1, B = 2 };
struct S {
    Kind kind;
    if (kind == Kind::A) { u8 forA; }
    if (kind == Kind::B) { u16 forB; }
};
S s @ 0x00;`,
        data: "02" + "0304",
    },

    // --- expressions ------------------------------------------------------
    {
        name: "expression_division_and_modulo",
        pattern: `struct S {
    u8 items[(10 / 3) + (10 % 3)];
};
S s @ 0x00;`,
        data: "0102030405060708",
    },
    {
        name: "expression_unary_minus_in_comparison",
        pattern: `struct S {
    s8 v;
    if (v == -1) { u8 flagged; }
};
S s @ 0x00;`,
        data: "ff" + "77",
    },
    {
        name: "expression_bitwise_or_and_xor",
        pattern: `struct S {
    u8 items[(1 | 2) ^ 1];
};
S s @ 0x00;`,
        data: "01020304",
    },
    {
        name: "expression_right_shift",
        pattern: `struct S {
    u8 items[64 >> 4];
};
S s @ 0x00;`,
        data: "01020304",
    },
    {
        name: "expression_parenthesised_precedence",
        pattern: `struct S {
    u8 flat[1 + 2 * 3];
    u8 grouped[(1 + 2) * 1];
};
S s @ 0x00;`,
        data: "0102030405060708090a",
    },
    {
        name: "sizeof_scalar_types",
        pattern: `struct S {
    u8 a[sizeof(u16)];
    u8 b[sizeof(u64)];
};
S s @ 0x00;`,
        data: "0102" + "030405060708090a",
    },
    {
        name: "sizeof_nested_struct",
        pattern: `struct Inner { u8 a; u8 b; };
struct Middle { Inner one; Inner two; };
struct S { u8 items[sizeof(Middle)]; };
S s @ 0x00;`,
        data: "0102030405060708",
    },

    // --- scalars at the edges ---------------------------------------------
    {
        name: "scalar_zero_values",
        pattern: `struct S {
    u8 a;
    u16 b;
    u32 c;
    s8 d;
    float e;
};
S s @ 0x00;`,
        data: "00" + "0000" + "00000000" + "00" + "00000000",
    },
    {
        name: "double_negative_zero_and_one",
        pattern: `struct S {
    double negative_zero;
    double one;
};
S s @ 0x00;`,
        data: "0000000000000080" + "000000000000f03f",
    },
    {
        name: "bool_nonzero_is_true",
        pattern: `struct S {
    bool zero;
    bool one;
    bool many;
};
S s @ 0x00;`,
        data: "00" + "01" + "7f",
    },
    {
        name: "char_non_ascii_bytes",
        pattern: `struct S { char letters[4]; };
S s @ 0x00;`,
        data: "41" + "7a" + "30" + "20",
    },
    {
        name: "signed_boundary_values",
        pattern: `struct S {
    s8 min;
    s8 max;
    s16 wide_min;
    s16 wide_max;
    s32 widest_min;
    s32 widest_max;
};
S s @ 0x00;`,
        data: "80" + "7f" + "0080" + "ff7f" + "00000080" + "ffffff7f",
    },

    // --- composites -------------------------------------------------------
    {
        name: "union_of_enum_and_scalar",
        pattern: `enum Kind : u8 { A = 1 };
union U { Kind kind; u8 raw; };
U u @ 0x00;`,
        data: "01",
    },
    {
        name: "union_containing_bitfield",
        pattern: `bitfield Flags { a : 4; b : 4; };
union U { Flags flags; u8 raw; };
U u @ 0x00;`,
        data: "5a",
    },
    {
        name: "union_containing_struct",
        pattern: `struct Inner { u8 a; u8 b; };
union U { Inner pair; u16 word; };
U u @ 0x00;`,
        data: "0102",
    },
    {
        name: "struct_of_one_member",
        pattern: `struct S { u8 only; };
S s @ 0x00;`,
        data: "42",
    },
    {
        name: "struct_with_trailing_array",
        pattern: `struct S {
    u16 header;
    u8 body[4];
};
S s @ 0x00;`,
        data: "0102" + "03040506",
    },
    {
        name: "array_of_unions",
        pattern: `union U { u16 word; u8 pair[2]; };
struct S { U items[2]; };
S s @ 0x00;`,
        data: "0102" + "0304",
    },
    {
        name: "array_of_nested_structs",
        pattern: `struct Leaf { u8 v; };
struct Branch { Leaf leaves[2]; };
struct S { Branch branches[2]; };
S s @ 0x00;`,
        data: "01020304",
    },
    {
        name: "enum_inside_nested_struct",
        pattern: `enum Kind : u8 { A = 1, B = 2 };
struct Inner { Kind kind; u8 v; };
struct S { Inner one; Inner two; };
S s @ 0x00;`,
        data: "0105" + "0206",
    },

    // --- aliases and endianness together -----------------------------------
    {
        name: "using_alias_of_bitfield",
        pattern: `bitfield Flags { a : 3; b : 5; };
using F = Flags;
struct S { F one; F two; };
S s @ 0x00;`,
        data: "a5" + "5a",
    },
    {
        name: "using_alias_little_endian",
        pattern: `using Word = le u32;
struct S { Word a; };
S s @ 0x00;`,
        data: "01020304",
    },
    {
        name: "endianness_on_array_elements",
        pattern: `struct S {
    be u16 items[3];
};
S s @ 0x00;`,
        data: "0102" + "0304" + "0506",
    },
    {
        name: "endianness_default_is_little",
        pattern: `struct S {
    u16 plain;
    le u16 explicit_little;
};
S s @ 0x00;`,
        data: "0102" + "0102",
    },
    {
        name: "endianness_inside_nested_struct",
        pattern: `struct Inner { be u32 v; };
struct S { Inner one; le u32 two; };
S s @ 0x00;`,
        data: "01020304" + "01020304",
    },
];

const outputDir = path.join(repoRoot, "crates/ferrosift-pattern/tests/fixtures");
mkdirSync(outputDir, {recursive: true});

const binary = reference();
const commit = execFileSync("git", ["-C", checkout, "rev-parse", "HEAD"], {
    encoding: "utf8",
}).trim();

// Rebuilt from empty each run. `plcli format` refuses to write over a file
// that is already there and says nothing about why, so a second run against a
// warm directory recorded no cases at all while reporting only that the
// reference had "refused" them -- a fixture that quietly shrank rather than a
// generator that stopped.
const temporary = path.join(os.tmpdir(), "ferrosift-pattern-oracle");
rmSync(temporary, {recursive: true, force: true});
mkdirSync(temporary, {recursive: true});

const recorded = [];
const refused = [];
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
        const detail = error?.stderr?.toString().trim() || error?.stdout?.toString().trim();
        refused.push(testCase.name);
        process.stderr.write(
            `reference refused ${testCase.name}: ${detail || `exit ${error?.status ?? "?"}`}\n`,
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
if (refused.length) {
    // Named rather than only counted. A case the reference will not run is a
    // fact about the language, and finding out which one it was should not
    // mean reading back through the log.
    process.stdout.write(`refused: ${refused.join(", ")}\n`);
}
