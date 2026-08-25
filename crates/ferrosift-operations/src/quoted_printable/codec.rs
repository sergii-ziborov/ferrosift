//! Quoted-printable, in both directions.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// The longest line quoted-printable permits, including the soft-break `=`.
const LINE_MAX: usize = 76;

/// How far back the encoder will look for a nicer place to break.
///
/// A third of the line, which is the reference's choice and is what makes the
/// output break on word boundaries rather than mid-word.
const MARGIN: usize = LINE_MAX / 3;

/// Bytes that survive unescaped.
///
/// Not simply "printable ASCII": the gaps are the interesting part. `"` and
/// `?` and `_` are escaped while every letter around them is not, and `=`
/// obviously is because it introduces an escape. These are the reference's
/// ranges rather than the RFC's, and the two are not identical.
fn is_literal(byte: u8) -> bool {
    matches!(byte,
        0x09 | 0x0A | 0x0D | 0x20 | 0x21
        | 0x23..=0x3C
        | 0x3E
        | 0x40..=0x5E
        | 0x60..=0x7E)
}

/// Encodes bytes as quoted-printable text.
#[must_use]
pub fn encode(input: &[u8]) -> String {
    soft_break(&escape_trailing_blanks(&normalise_endings(&escape(input))))
}

/// Replaces every byte outside the literal ranges with `=XX`.
fn escape(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input {
        if is_literal(*byte) {
            output.push(char::from(*byte));
        } else {
            output.push('=');
            output.push(hex_digit(byte >> 4));
            output.push(hex_digit(byte & 0x0f));
        }
    }
    output
}

/// Upper-case, which is what the reference emits.
fn hex_digit(value: u8) -> char {
    char::from(if value < 10 {
        b'0' + value
    } else {
        b'A' + value - 10
    })
}

/// Rewrites every line ending as CRLF.
fn normalise_endings(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\r' => {
                output.push_str("\r\n");
                at += if bytes.get(at + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
            }
            b'\n' => {
                output.push_str("\r\n");
                at += 1;
            }
            other => {
                output.push(char::from(other));
                at += 1;
            }
        }
    }
    output
}

/// Escapes runs of spaces and tabs that sit at the end of a line.
///
/// A mail transfer agent is free to strip trailing whitespace, so leaving it
/// literal would let the message change in transit. Escaping it is what makes
/// quoted-printable able to carry it at all.
fn escape_trailing_blanks(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b' ' || bytes[at] == b'\t' {
            let mut end = at;
            while end < bytes.len() && (bytes[end] == b' ' || bytes[end] == b'\t') {
                end += 1;
            }
            // Only a run that reaches a line terminator or the end of the
            // input is trailing; one in the middle of a line is left alone.
            let trailing = end == bytes.len() || bytes[end] == b'\r' || bytes[end] == b'\n';
            for byte in &bytes[at..end] {
                if trailing {
                    output.push_str(if *byte == b' ' { "=20" } else { "=09" });
                } else {
                    output.push(char::from(*byte));
                }
            }
            at = end;
            continue;
        }
        output.push(char::from(bytes[at]));
        at += 1;
    }
    output
}

/// Inserts soft line breaks so no line exceeds the limit.
///
/// This is the reference's algorithm rather than a reimplementation of the
/// RFC, and the difference shows. It prefers to break after an existing line
/// ending, then at a word boundary within the last quarter of the line, and
/// only then mechanically — and it takes care never to split an `=XX` escape,
/// nor to split a multi-byte UTF-8 sequence that is written as consecutive
/// escapes, which the RFC does not require and readers appreciate.
fn soft_break(input: &str) -> String {
    let bytes = input.as_bytes();
    let total = bytes.len();
    let mut output = String::with_capacity(total + total / LINE_MAX * 3);
    let mut pos = 0;

    while pos < total {
        let mut line = &input[pos..total.min(pos + LINE_MAX)];

        // An existing hard break inside the window ends the line there.
        if let Some(at) = line.find("\r\n") {
            line = &line[..at + 2];
            output.push_str(line);
            pos += line.len();
            continue;
        }
        if line.ends_with('\n') {
            output.push_str(line);
            pos += line.len();
            continue;
        }

        let tail = &line[line.len() - MARGIN.min(line.len())..];
        if let Some(at) = tail.rfind('\n') {
            // Truncate back to just after the last newline in the tail.
            let after = tail.len() - at - 1;
            line = &line[..line.len() - after];
            output.push_str(line);
            pos += line.len();
            continue;
        }

        // One condition, not two: a long line whose tail holds no separator
        // does not stop here, it falls through to the escape handling below.
        // Splitting this into a length test and a nested search would silently
        // skip that, and a line made entirely of `=XX` escapes — which has no
        // separator by construction — is exactly the case that reaches it.
        let separator = if line.len() > LINE_MAX - MARGIN {
            tail.rfind([' ', '\t', '.', ',', '!', '?'])
        } else {
            None
        };
        if let Some(at) = separator {
            // Keep the separator, move what follows to the next line.
            let after = tail.len() - at - 1;
            line = &line[..line.len() - after];
        } else if line.ends_with('\r') {
            line = &line[..line.len() - 1];
        } else {
            line = keep_escapes_whole(line, total - pos);
        }

        if pos + line.len() < total && !line.ends_with('\n') {
            if line.len() == LINE_MAX {
                // Room has to be made for the trailing `=`, and an escape must
                // not be cut in half to make it.
                let drop = if ends_with_escape(line) { 3 } else { 1 };
                line = &line[..line.len() - drop];
            }
            pos += line.len();
            output.push_str(line);
            output.push_str("=\r\n");
        } else {
            pos += line.len();
            output.push_str(line);
        }
    }

    output
}

/// Whether the line ends in a complete `=XX` escape.
fn ends_with_escape(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 3
        && bytes[bytes.len() - 3] == b'='
        && bytes[bytes.len() - 2].is_ascii_hexdigit()
        && bytes[bytes.len() - 1].is_ascii_hexdigit()
}

/// Pulls a partial escape, and a split UTF-8 sequence, onto the next line.
fn keep_escapes_whole(line: &str, remaining: usize) -> &str {
    let mut line = line;
    if !ends_in_partial_or_full_escape(line) {
        return line;
    }

    // An `=` or `=X` at the end is not an escape yet; it belongs to the next
    // line in one piece.
    if let Some(width) = partial_escape_width(line) {
        line = &line[..line.len() - width];
    }

    // Then walk back over whole escapes while they look like the tail of a
    // multi-byte UTF-8 sequence, so the sequence moves as a unit.
    while line.len() > 3
        && line.len() < remaining
        && !is_only_escapes(line)
        && ends_with_escape(line)
    {
        let code = escape_value(line);
        if code < 0x80 {
            break;
        }
        line = &line[..line.len() - 3];
        if code >= 0xC0 {
            // A lead byte: everything after it has now moved too.
            break;
        }
    }
    line
}

/// Whether the line ends with `=`, `=X`, or `=XX`.
fn ends_in_partial_or_full_escape(line: &str) -> bool {
    partial_escape_width(line).is_some() || ends_with_escape(line)
}

/// The width of a trailing `=` or `=X`, if there is one.
fn partial_escape_width(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    if bytes.last() == Some(&b'=') {
        return Some(1);
    }
    if bytes.len() >= 2
        && bytes[bytes.len() - 2] == b'='
        && bytes[bytes.len() - 1].is_ascii_hexdigit()
    {
        return Some(2);
    }
    None
}

/// Whether the line is nothing but one to four escapes.
///
/// The guard that stops the loop above from emptying a line that is only a
/// multi-byte character: there would be nothing left to break.
fn is_only_escapes(line: &str) -> bool {
    let bytes = line.as_bytes();
    if !bytes.len().is_multiple_of(3) || bytes.is_empty() || bytes.len() > 12 {
        return false;
    }
    bytes.chunks_exact(3).all(|chunk| {
        chunk[0] == b'=' && chunk[1].is_ascii_hexdigit() && chunk[2].is_ascii_hexdigit()
    })
}

/// The byte value of the escape a line ends with.
fn escape_value(line: &str) -> u32 {
    let bytes = line.as_bytes();
    let high = char::from(bytes[bytes.len() - 2]).to_digit(16).unwrap_or(0);
    let low = char::from(bytes[bytes.len() - 1]).to_digit(16).unwrap_or(0);
    high * 16 + low
}

/// Decodes quoted-printable text into bytes.
///
/// Soft line breaks — an `=` at end of line, or at the very end of the input —
/// are removed first. After that an `=` followed by two hex digits is one
/// byte, and anything else is its own code unit.
///
/// # Errors
///
/// Returns an error when a literal character is above `U+00FF`, which is not a
/// byte and which the reference also fails to convert.
pub fn decode(input: &str) -> Result<Vec<u8>, OperationError> {
    // `=(?:\r?\n|$)` removed globally, including the end-of-input case.
    let mut joined = String::with_capacity(input.len());
    let characters: Vec<char> = input.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '=' {
            if index + 1 >= characters.len() {
                index += 1;
                continue;
            }
            if characters[index + 1] == '\n' {
                index += 2;
                continue;
            }
            if characters[index + 1] == '\r'
                && index + 2 < characters.len()
                && characters[index + 2] == '\n'
            {
                index += 3;
                continue;
            }
        }
        joined.push(characters[index]);
        index += 1;
    }

    let symbols: Vec<char> = joined.chars().collect();
    let mut output = Vec::with_capacity(symbols.len());
    let mut cursor = 0;
    while cursor < symbols.len() {
        if symbols[cursor] == '=' && cursor + 2 < symbols.len() {
            let high = symbols[cursor + 1];
            let low = symbols[cursor + 2];
            if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() {
                let value = high.to_digit(16).unwrap_or(0) * 16 + low.to_digit(16).unwrap_or(0);
                output.push(u8::try_from(value).unwrap_or(0));
                cursor += 3;
                continue;
            }
        }
        // `charCodeAt` gives a UTF-16 code unit, which is not a byte above
        // U+00FF. The reference then fails converting the result, so refusing
        // here says the same thing up front.
        let code = symbols[cursor] as u32;
        output
            .push(u8::try_from(code).map_err(|_| failed("encoding.quoted_printable.not_a_byte"))?);
        cursor += 1;
    }
    Ok(output)
}
