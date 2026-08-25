use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// What to do with a character the smart map does not cover.
#[derive(Clone, Copy)]
pub(super) enum Unmappable {
    Include,
    Remove,
    Dot,
}

pub(super) fn unmappable(value: &str) -> Result<Unmappable, OperationError> {
    match value {
        "Include" => Ok(Unmappable::Include),
        "Remove" => Ok(Unmappable::Remove),
        "Replace with '.'" => Ok(Unmappable::Dot),
        _ => Err(failed("text.smart.invalid_unmappable")),
    }
}

/// Typographic characters and their ASCII stand-ins.
///
/// The reference's own table, in its own order. Several characters map to the
/// same replacement and two map to more than one character, so this is a
/// lookup rather than an arithmetic transform.
const SMART_MAP: &[(char, &str)] = &[
    // Smart double quotes
    ('\u{201C}', "\""),
    ('\u{201D}', "\""),
    ('\u{201E}', "\""),
    ('\u{201F}', "\""),
    ('\u{2033}', "\""),
    // Smart single quotes and apostrophes
    ('\u{2018}', "'"),
    ('\u{2019}', "'"),
    ('\u{201A}', "'"),
    ('\u{201B}', "'"),
    ('\u{2032}', "'"),
    // Dashes and hyphens
    ('\u{2010}', "-"),
    ('\u{2011}', "-"),
    ('\u{2012}', "-"),
    ('\u{2013}', "-"),
    ('\u{2014}', "--"),
    ('\u{2015}', "--"),
    // Ellipsis
    ('\u{2026}', "..."),
    // Trademark and copyright
    ('\u{00A9}', "(c)"),
    ('\u{00AE}', "(r)"),
    ('\u{2122}', "(tm)"),
    // Arrows
    ('\u{2190}', "<--"),
    ('\u{2192}', "-->"),
    ('\u{2191}', "^"),
    ('\u{2193}', "v"),
    ('\u{2194}', "<->"),
    ('\u{21D0}', "<=="),
    ('\u{21D2}', "==>"),
    ('\u{21D4}', "<=>"),
    // Guillemets
    ('\u{00AB}', "<<"),
    ('\u{00BB}', ">>"),
    ('\u{2039}', "<"),
    ('\u{203A}', ">"),
    // Maths and miscellaneous
    ('\u{00D7}', "x"),
    ('\u{00F7}', "/"),
    ('\u{00B1}', "+/-"),
    ('\u{2022}', "*"),
    ('\u{00B7}', "."),
    // Non-ASCII spaces
    ('\u{00A0}', " "),
    ('\u{2002}', " "),
    ('\u{2003}', " "),
    ('\u{2009}', " "),
    ('\u{200A}', " "),
];

/// Replaces typographic characters with ASCII equivalents.
///
/// ASCII passes through before the table is consulted, exactly as the
/// reference checks `codePointAt(0) < 128` first — which matters because the
/// table would otherwise never be reached for them anyway, but the ordering is
/// what decides the `Remove` and `.` branches for ASCII input.
pub(super) fn escape_smart(input: &str, unmappable: Unmappable) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        if (character as u32) < 128 {
            output.push(character);
            continue;
        }
        if let Some((_, replacement)) = SMART_MAP.iter().find(|(from, _)| *from == character) {
            output.push_str(replacement);
            continue;
        }
        match unmappable {
            Unmappable::Include => output.push(character),
            Unmappable::Remove => {}
            Unmappable::Dot => output.push('.'),
        }
    }
    output
}

/// Removes HTML tags, then optionally tidies the whitespace left behind.
pub(super) fn strip_html_tags(
    input: &str,
    remove_indentation: bool,
    remove_line_breaks: bool,
) -> String {
    let mut text = remove_tags(input);
    if remove_indentation {
        text = remove_indent(&text);
    }
    if remove_line_breaks {
        text = collapse_blank_lines(&text);
    }
    text
}

/// Strips `<…>` runs until none are left.
///
/// The reference repeats the removal rather than doing one pass, and says why:
/// a single pass over `aabcbc` with `abc` removed leaves `abc` behind. The
/// same is true of nested angle brackets here, so the loop is the behaviour
/// rather than an optimisation.
fn remove_tags(input: &str) -> String {
    let mut current = String::from(input);
    loop {
        let stripped = remove_tags_once(&current);
        if stripped.len() == current.len() {
            return stripped;
        }
        current = stripped;
    }
}

fn remove_tags_once(input: &str) -> String {
    let characters: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < characters.len() {
        if characters[index] != '<' {
            output.push(characters[index]);
            index += 1;
            continue;
        }
        // `<[^>]+>` needs at least one non-`>` character before the close.
        let mut cursor = index + 1;
        while cursor < characters.len() && characters[cursor] != '>' {
            cursor += 1;
        }
        if cursor < characters.len() && cursor > index + 1 {
            index = cursor + 1;
        } else {
            output.push(characters[index]);
            index += 1;
        }
    }
    output
}

/// `\n[ \f\t]+` becomes `\n`.
fn remove_indent(input: &str) -> String {
    let characters: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < characters.len() {
        output.push(characters[index]);
        if characters[index] == '\n' {
            let mut cursor = index + 1;
            while cursor < characters.len() && matches!(characters[cursor], ' ' | '\u{0C}' | '\t') {
                cursor += 1;
            }
            index = cursor;
        } else {
            index += 1;
        }
    }
    output
}

/// Drops a leading blank line, then collapses runs of them.
///
/// The reference applies `^\s*\n` once — no global flag, so only at the very
/// start — and then `(\n\s*){2,}` globally. The second needs two or more
/// newline-plus-whitespace groups, which after backtracking means: a
/// whitespace run beginning with a newline and containing at least two of
/// them collapses to one newline. A run with a single newline is left alone.
fn collapse_blank_lines(input: &str) -> String {
    let characters: Vec<char> = input.chars().collect();

    // `^\s*\n`, greedy: everything up to and including the last newline in the
    // leading whitespace run.
    let mut start = 0;
    let mut last_newline = None;
    while start < characters.len() && characters[start].is_whitespace() {
        if characters[start] == '\n' {
            last_newline = Some(start);
        }
        start += 1;
    }
    let begin = last_newline.map_or(0, |index| index + 1);

    let mut output = String::with_capacity(input.len());
    let mut index = begin;
    while index < characters.len() {
        if characters[index] != '\n' {
            output.push(characters[index]);
            index += 1;
            continue;
        }
        let mut cursor = index;
        let mut newlines = 0usize;
        while cursor < characters.len() && characters[cursor].is_whitespace() {
            if characters[cursor] == '\n' {
                newlines += 1;
            }
            cursor += 1;
        }
        if newlines >= 2 {
            output.push('\n');
            index = cursor;
        } else {
            output.push(characters[index]);
            index += 1;
        }
    }
    output
}
