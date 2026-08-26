//! Rendering delimited text as a table.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

/// Code units the renderers spell out, named so the tables read as tables.
const SPACE: u16 = 0x20;
const PIPE: u16 = 0x7c;
const DASH: u16 = 0x2d;
const CROSS: u16 = 0x2b;
const NEWLINE: u16 = 0x0a;

/// Which of the three renderings to produce.
#[derive(Clone, Copy)]
pub(super) enum Format {
    Ascii,
    Html,
    Markdown,
}

/// Splits the input into cells and renders it.
///
/// Escaping happens *before* parsing, which is not a detail: a quote in the
/// input has become `&quot;` by the time the parser looks for one, so the
/// parser's quoted-field handling can never fire and a quoted CSV field is
/// read as its literal characters. The same goes for a delimiter that is an
/// HTML-special character -- asking to split on `<` splits on nothing, because
/// no `<` survives to be found.
pub(super) fn render(
    input: &str,
    cell_delimiters: &str,
    row_delimiters: &str,
    header: bool,
    format: Format,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let escaped = escape_html(input);
    let cells: Vec<u16> = cell_delimiters.encode_utf16().collect();
    let rows: Vec<u16> = row_delimiters.encode_utf16().collect();
    let mut table = parse_delimited(&escaped, &cells, &rows, context)?;

    if table.is_empty() {
        return Ok(String::new());
    }

    let widths = column_widths(&table);
    Ok(match format {
        Format::Ascii => ascii(&mut table, &widths, header),
        Format::Html => html(&mut table, header),
        Format::Markdown => markdown(&mut table, &widths),
    })
}

/// The reference's own escape table, applied to the whole input at once.
fn escape_html(input: &str) -> Vec<u16> {
    let mut output = Vec::with_capacity(input.len());
    for unit in input.encode_utf16() {
        match unit {
            0x26 => push(&mut output, "&amp;"),
            0x3c => push(&mut output, "&lt;"),
            0x3e => push(&mut output, "&gt;"),
            0x22 => push(&mut output, "&quot;"),
            0x27 => push(&mut output, "&#x27;"),
            0x60 => push(&mut output, "&#x60;"),
            0x00 => push(&mut output, "\u{e000}"),
            _ => output.push(unit),
        }
    }
    output
}

/// Splits escaped text into rows of cells.
///
/// A row is only recorded once it holds at least one cell, so input carrying
/// no delimiter at all produces no rows and the operation returns nothing --
/// a single unsplit value is not a table, and the reference decides that here
/// rather than by checking the input.
fn parse_delimited(
    data: &[u16],
    cell_delimiters: &[u16],
    row_delimiters: &[u16],
    context: &OperationContext<'_>,
) -> Result<Vec<Vec<Vec<u16>>>, OperationError> {
    // A byte-order mark is dropped rather than becoming part of the first
    // cell, because a spreadsheet routinely writes one.
    let data = match data.split_first() {
        Some((0xfeff, rest)) => rest,
        _ => data,
    };

    let mut lines: Vec<Vec<Vec<u16>>> = Vec::new();
    let mut line: Vec<Vec<u16>> = Vec::new();
    let mut cell: Vec<u16> = Vec::new();
    let mut in_string = false;
    let mut render_next = false;

    let mut index = 0;
    while index < data.len() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        let unit = data[index];
        let next = data.get(index + 1).copied();

        if render_next {
            cell.push(unit);
            render_next = false;
        } else if unit == 0x22 && !in_string {
            in_string = true;
        } else if unit == 0x22 {
            if next == Some(0x22) {
                render_next = true;
            } else {
                in_string = false;
            }
        } else if !in_string && cell_delimiters.contains(&unit) {
            line.push(core::mem::take(&mut cell));
        } else if !in_string && row_delimiters.contains(&unit) {
            line.push(core::mem::take(&mut cell));
            lines.push(core::mem::take(&mut line));
            // A different second delimiter is consumed with the first, so a
            // `\r\n` pair ends one row rather than one row and an empty one.
            // Two of the *same* character stay two endings.
            if next.is_some_and(|next| row_delimiters.contains(&next) && next != unit) {
                index += 1;
            }
        } else {
            cell.push(unit);
        }
        index += 1;
    }

    if !line.is_empty() {
        line.push(cell);
        lines.push(line);
    }
    Ok(lines)
}

/// The widest cell in each column, measured before any row is removed.
fn column_widths(table: &[Vec<Vec<u16>>]) -> Vec<usize> {
    let mut widths: Vec<usize> = Vec::new();
    for row in table {
        for (column, cell) in row.iter().enumerate() {
            if column >= widths.len() {
                widths.resize(column + 1, 0);
            }
            widths[column] = widths[column].max(cell.len());
        }
    }
    widths
}

/// Boxed borders and padded cells.
fn ascii(table: &mut Vec<Vec<Vec<u16>>>, widths: &[usize], header: bool) -> String {
    let mut output = String::new();
    output.push_str(&horizontal_border(widths));
    if header && !table.is_empty() {
        let row = table.remove(0);
        output.push_str(&padded_row(&row, widths));
        output.push_str(&horizontal_border(widths));
    }
    for row in table.iter() {
        output.push_str(&padded_row(row, widths));
    }
    output.push_str(&horizontal_border(widths));
    output
}

/// `+---+---+` sized to the columns.
fn horizontal_border(widths: &[usize]) -> String {
    let mut units: Vec<u16> = alloc::vec![CROSS];
    for width in widths {
        // Two wider than the cell, for the space on each side of it.
        repeat(&mut units, DASH, width + 2);
        units.push(CROSS);
    }
    units.push(NEWLINE);
    finish(&units)
}

/// `| a | b |`, each cell padded to its column.
fn padded_row(row: &[Vec<u16>], widths: &[usize]) -> String {
    let mut units: Vec<u16> = alloc::vec![PIPE];
    for (column, cell) in row.iter().enumerate() {
        units.push(SPACE);
        units.extend_from_slice(cell);
        let padding = widths
            .get(column)
            .copied()
            .unwrap_or(0)
            .saturating_sub(cell.len());
        repeat(&mut units, SPACE, padding);
        units.push(SPACE);
        units.push(PIPE);
    }
    units.push(NEWLINE);
    finish(&units)
}

/// Appends `count` copies of one code unit.
fn repeat(output: &mut Vec<u16>, unit: u16, count: usize) {
    output.extend(core::iter::repeat_n(unit, count));
}

/// A `<table>` carrying the reference's own styling classes.
fn html(table: &mut Vec<Vec<Vec<u16>>>, header: bool) -> String {
    let mut output =
        String::from("<table class='table table-hover table-sm table-bordered table-nonfluid'>");
    if header && !table.is_empty() {
        let row = table.remove(0);
        output.push_str("<thead class='thead-light'>");
        output.push_str(&tag_row(&row, "th"));
        output.push_str("</thead>");
    }
    output.push_str("<tbody>");
    for row in table.iter() {
        output.push_str(&tag_row(row, "td"));
    }
    output.push_str("</tbody></table>");
    output
}

/// One `<tr>` of cells in the given tag.
fn tag_row(row: &[Vec<u16>], cell_tag: &str) -> String {
    let mut units: Vec<u16> = Vec::new();
    push(&mut units, "<tr>");
    for cell in row {
        push(&mut units, "<");
        push(&mut units, cell_tag);
        push(&mut units, ">");
        units.extend_from_slice(cell);
        push(&mut units, "</");
        push(&mut units, cell_tag);
        push(&mut units, ">");
    }
    push(&mut units, "</tr>");
    finish(&units)
}

/// A pipe table, whose first row is always the header.
///
/// The header flag is not consulted: the reference removes the first row
/// regardless, on the grounds that the renderer it targets will not display a
/// table without one. Asking for no header therefore still spends the first
/// row on the header, which is why the flag is absent here rather than passed
/// in and ignored.
fn markdown(table: &mut Vec<Vec<Vec<u16>>>, widths: &[usize]) -> String {
    let mut output = String::new();
    if table.is_empty() {
        return output;
    }
    let row = table.remove(0);
    output.push_str(&padded_row(&row, widths));

    let mut divider: Vec<u16> = alloc::vec![PIPE];
    for column in 0..row.len() {
        divider.push(SPACE);
        repeat(&mut divider, DASH, widths.get(column).copied().unwrap_or(0));
        divider.push(SPACE);
        divider.push(PIPE);
    }
    divider.push(NEWLINE);
    output.push_str(&finish(&divider));

    for row in table.iter() {
        output.push_str(&padded_row(row, widths));
    }
    output
}

/// Appends an ASCII string as code units.
fn push(output: &mut Vec<u16>, text: &str) {
    output.extend(text.encode_utf16());
}

/// Turns accumulated code units back into text.
///
/// Every unit here either came from the input, which was valid text, or from
/// an ASCII literal, so a pair can only be broken if the input carried a lone
/// surrogate -- which cannot survive being read as a Rust string in the first
/// place.
fn finish(units: &[u16]) -> String {
    String::from_utf16(units).unwrap_or_default()
}

/// Resolves the format an argument names.
///
/// Total rather than fallible: the reference's `switch` sends anything it does
/// not recognise to the HTML branch, so there is no name this can refuse.
pub(super) fn format(value: &str) -> Format {
    match value {
        "ASCII" => Format::Ascii,
        "Markdown" => Format::Markdown,
        _ => Format::Html,
    }
}
