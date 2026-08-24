// Line, whitespace, and byte-selection operations.
//
// These are the operations where JavaScript's own string semantics leak into
// the result — UTF-16 padding widths, the multiline `^` anchor, `split("")`
// on an empty separator — so the sampled inputs deliberately include the
// shapes that expose them: empty input, no trailing newline, blank lines, and
// astral characters.

const LINE_TEXTS = [
    "",
    "one",
    "one\ntwo\nthree",
    "one\ntwo\nthree\n",
    "\n\n",
    "  12. indented\n3) second\n     456| deep\n      7 too deep",
    "a\r\nb\r\nc",
    "1 alpha\n2 beta\n3 gamma",
];

const PAD_TEXTS = ["", "ab\ncde", "\n", "héllo\nwörld", "🙂\n🙂🙂"];

export function add({addCase, randomAscii, randomBytes}) {
    LINE_TEXTS.forEach((value, index) => {
        addCase(`tail_lines_${index}`, value, [{op: "Tail", args: ["Line feed", 2]}]);
        addCase(`tail_negative_${index}`, value, [{op: "Tail", args: ["Line feed", -1]}]);
        addCase(`add_line_numbers_${index}`, value, [{op: "Add line numbers", args: [0]}]);
        addCase(`add_line_numbers_offset_${index}`, value, [
            {op: "Add line numbers", args: [98]},
        ]);
        addCase(`remove_line_numbers_${index}`, value, [{op: "Remove line numbers", args: []}]);
        addCase(`line_numbers_round_trip_${index}`, value, [
            {op: "Add line numbers", args: [0]},
            {op: "Remove line numbers", args: []},
        ]);
    });

    // Tail over a non-newline delimiter, to pin charRep rather than only "\n".
    addCase("tail_comma", "a,b,c,d,e", [{op: "Tail", args: ["Comma", 3]}]);
    addCase("tail_chars", "abcdef", [{op: "Tail", args: ["Nothing (separate chars)", 2]}]);

    PAD_TEXTS.forEach((value, index) => {
        addCase(`pad_start_${index}`, value, [{op: "Pad lines", args: ["Start", 5, " "]}]);
        addCase(`pad_end_${index}`, value, [{op: "Pad lines", args: ["End", 3, "-"]}]);
        addCase(`pad_multi_${index}`, value, [{op: "Pad lines", args: ["Start", 5, "ab"]}]);
        addCase(`pad_zero_${index}`, value, [{op: "Pad lines", args: ["End", 0, "x"]}]);
    });

    for (const length of [0, 4, 20]) {
        const raw = randomAscii(length).toString("latin1");
        addCase(`remove_whitespace_default_${length}`, ` ${raw}\t\r\n\f.${raw} `, [
            {op: "Remove whitespace", args: [true, true, true, true, true, false]},
        ]);
        addCase(`remove_whitespace_stops_${length}`, ` ${raw}\t\r\n\f.${raw} `, [
            {op: "Remove whitespace", args: [false, false, false, false, false, true]},
        ]);
    }

    for (const length of [0, 1, 9, 64]) {
        const raw = randomBytes(length);
        addCase(`remove_null_bytes_${length}`, Buffer.concat([raw, Buffer.from([0, 0])]), [
            {op: "Remove null bytes", args: []},
        ]);
        addCase(`reverse_byte_${length}`, raw, [{op: "Reverse", args: ["Byte"]}]);
        addCase(`reverse_line_${length}`, raw, [{op: "Reverse", args: ["Line"]}]);
        for (const [every, start] of [[4, 0], [3, 2], [1, 0], [5, 7]]) {
            addCase(`take_nth_${length}_${every}_${start}`, raw, [
                {op: "Take nth bytes", args: [every, start, false]},
            ]);
            addCase(`drop_nth_${length}_${every}_${start}`, raw, [
                {op: "Drop nth bytes", args: [every, start, false]},
            ]);
        }
    }

    // Per-line mode restarts the offset, so it needs input with line feeds.
    const lined = Buffer.from("abcdefgh\nijkl\n\nmnopqrstuv", "latin1");
    addCase("take_nth_each_line", lined, [{op: "Take nth bytes", args: [3, 1, true]}]);
    addCase("drop_nth_each_line", lined, [{op: "Drop nth bytes", args: [3, 1, true]}]);
    addCase("reverse_line_text", lined, [{op: "Reverse", args: ["Line"]}]);

    // Character mode walks UTF-16, so multi-byte and astral input is the point.
    // Reverse takes bytes, so these are fed as UTF-8 rather than as text.
    for (const [name, value] of [
        ["ascii", "abcdef"],
        ["latin", "héllo wörld"],
        ["cjk", "日本語テキスト"],
        ["astral", "a🙂b🇬🇧c"],
        ["empty", ""],
    ]) {
        addCase(`reverse_character_${name}`, Buffer.from(value, "utf8"), [
            {op: "Reverse", args: ["Character"]},
        ]);
    }
}
