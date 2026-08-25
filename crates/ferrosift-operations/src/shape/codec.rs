use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

/// Removes ANSI control sequences.
///
/// The reference matches `\x1B\[[0-?]*[ -/]*[@-~]`: an escape, a bracket, then
/// parameter bytes, then intermediate bytes, then one final byte. A sequence
/// that runs out of input before its final byte is not a match, so its
/// characters stay in the output — which is why the scan holds a start index
/// and rewinds to it rather than dropping as it goes.
pub(super) fn remove_ansi(input: &str) -> String {
    let characters: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < characters.len() {
        if characters[index] != '\u{1b}'
            || index + 1 >= characters.len()
            || characters[index + 1] != '['
        {
            output.push(characters[index]);
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        while cursor < characters.len() && matches!(characters[cursor], '\u{30}'..='\u{3f}') {
            cursor += 1;
        }
        while cursor < characters.len() && matches!(characters[cursor], '\u{20}'..='\u{2f}') {
            cursor += 1;
        }
        if cursor < characters.len() && matches!(characters[cursor], '\u{40}'..='\u{7e}') {
            // Complete sequence: skip it, final byte included.
            index = cursor + 1;
        } else {
            // Incomplete: the reference's regex fails here and the escape is
            // emitted like any other character.
            output.push(characters[index]);
            index += 1;
        }
    }
    output
}

/// Drops everything up to and including the first blank line.
///
/// The reference prefers a CRLF pair and falls back to a bare LF pair. Its
/// fallback keeps a quirk worth reproducing: when neither is present,
/// `indexOf` returns -1 and the offset becomes 1, which the guard then reads
/// as "no header found" and returns the input untouched. Writing that as an
/// explicit "not found" would behave the same and describe it better.
pub(super) fn strip_http_headers(input: &str) -> &str {
    if let Some(position) = input.find("\r\n\r\n") {
        return &input[position + 4..];
    }
    match input.find("\n\n") {
        Some(position) => &input[position + 2..],
        None => input,
    }
}

/// Reassembles a chunked HTTP body.
///
/// Chunk sizes are read with JavaScript's `parseInt` in base 16, which takes
/// the leading hex digits and ignores the rest of the line — that is what lets
/// a size line carry chunk extensions after a semicolon. A line with no
/// leading hex digit ends the body, as does a zero-length chunk.
pub(super) fn dechunk(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let mut remaining = input;
    let mut output = String::new();

    // The reference decides CRLF against LF once, from the first size line,
    // and uses that length for every chunk after it.
    let first_line_end = remaining.find('\n').map_or(0, |index| index + 1);
    let ending_length = if first_line_end >= 2 && remaining[..first_line_end].ends_with("\r\n") {
        2
    } else {
        1
    };
    let ending = if ending_length == 2 { "\r\n" } else { "\n" };

    let mut size_end = first_line_end;
    loop {
        context.ensure_active()?;
        let Some(size) = chunk_size(&remaining[..size_end]) else {
            break;
        };
        if size == 0 {
            break;
        }
        // A size larger than what is left is truncated by the reference's
        // slice rather than rejected, so the same bound applies here.
        let start = size_end.min(remaining.len());
        let end = start.saturating_add(size).min(remaining.len());
        output.push_str(&remaining[start..end]);

        let advance = size_end
            .saturating_add(size)
            .saturating_add(ending_length)
            .min(remaining.len());
        remaining = &remaining[advance..];
        size_end = remaining
            .find(ending)
            .map_or(0, |index| index + ending_length);
    }
    context.ensure_active()?;
    Ok(output)
}

/// A chunk size, or `None` where `parseInt` would give `NaN`.
///
/// The shared `jsint` helper saturates at a million, which is the right
/// answer for an operation whose argument is a count and the wrong one for a
/// byte length — a two-megabyte chunk would be silently truncated. The rules
/// that matter here are narrow enough to state directly: skip leading
/// whitespace, take the longest run of hex digits, ignore whatever follows,
/// and give nothing when there are no digits at all.
fn chunk_size(line: &str) -> Option<usize> {
    let mut characters = line.chars().skip_while(|value| value.is_whitespace());
    let mut value: usize = 0;
    let mut digits = 0usize;
    for symbol in &mut characters {
        let Some(digit) = symbol.to_digit(16) else {
            break;
        };
        value = value.checked_mul(16)?.checked_add(digit as usize)?;
        digits += 1;
    }
    (digits > 0).then_some(value)
}

/// Breaks the input into lines of at most `width` characters.
///
/// The reference uses `.{1,width}` with the global flag and joins on newline.
/// The dot does not match a line feed without the `s` flag, so existing line
/// feeds are not carried into any match and simply vanish, replaced by the
/// joins. Reproducing that means splitting on line feeds first and wrapping
/// each piece — and dropping empty pieces, because a zero-length run produces
/// no match at all.
pub(super) fn wrap(input: &str, width: usize) -> String {
    if input.is_empty() {
        return String::new();
    }
    let mut pieces: Vec<String> = Vec::new();
    for line in input.split('\n') {
        let characters: Vec<char> = line.chars().collect();
        for chunk in characters.chunks(width) {
            pieces.push(chunk.iter().collect());
        }
    }
    pieces.join("\n")
}
