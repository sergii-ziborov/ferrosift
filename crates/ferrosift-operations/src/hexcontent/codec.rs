use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_CONVERT: &str = "encoding.hex_content.invalid_convert";

/// Which bytes to lift out of the text and render as hex.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Convert {
    Special,
    SpecialAndSpaces,
    All,
}

impl Convert {
    pub(super) fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "Only special chars" => Ok(Self::Special),
            "Only special chars including spaces" => Ok(Self::SpecialAndSpaces),
            "All chars" => Ok(Self::All),
            _ => Err(failed(INVALID_CONVERT)),
        }
    }
}

/// Whether a byte is left as a character or moved into a hex run.
///
/// The kept set is exactly the digits, upper-case letters, lower-case letters,
/// and the space — read off the reference's chain of range comparisons rather
/// than from any definition of "special". Space is kept or not depending on the
/// mode, and is the one byte the two modes disagree about.
fn is_literal(byte: u8, convert: Convert) -> bool {
    if byte == b' ' {
        return convert != Convert::SpecialAndSpaces;
    }
    byte.is_ascii_digit() || byte.is_ascii_uppercase() || byte.is_ascii_lowercase()
}

/// Renders bytes in the SNORT hex-content notation.
///
/// Runs of non-literal bytes are wrapped in pipes, so `foo=bar` becomes
/// `foo|3d|bar`. Consecutive hex bytes share one pair of pipes, optionally
/// separated by spaces.
pub(super) fn encode(
    input: &[u8],
    convert: Convert,
    spaces: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if convert == Convert::All {
        let mut output = String::from("|");
        for (index, byte) in input.iter().enumerate() {
            if index.is_multiple_of(4096) {
                context.ensure_active()?;
            }
            if spaces && index > 0 {
                output.push(' ');
            }
            push_hex(&mut output, *byte);
        }
        output.push('|');
        return Ok(output);
    }

    let mut output = String::new();
    let mut in_hex = false;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if is_literal(*byte, convert) {
            if in_hex {
                output.push('|');
                in_hex = false;
            }
            output.push(char::from(*byte));
        } else {
            if in_hex {
                if spaces {
                    output.push(' ');
                }
            } else {
                output.push('|');
                in_hex = true;
            }
            push_hex(&mut output, *byte);
        }
    }
    if in_hex {
        output.push('|');
    }
    context.ensure_active()?;
    Ok(output)
}

fn push_hex(output: &mut String, byte: u8) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    output.push(char::from(DIGITS[usize::from(byte >> 4)]));
    output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
}

/// Reads the SNORT hex-content notation back into bytes.
///
/// A run of at least two hex digits or spaces between pipes is decoded; every
/// other character, pipes included, is passed through as its own byte. The
/// reference has a branch for a run that is not valid hex, but it cannot be
/// reached — the pattern already admits only hex and spaces, and the check
/// guarding it tests an array, which is always truthy in JavaScript. There is
/// no behaviour there to reproduce, so there is none here.
///
/// Characters outside Latin-1 cannot be one byte, and the reference's `ord`
/// would keep only the low half of the code unit. Refusing is the honest
/// reading: the input was not this format.
pub(super) fn decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let characters: Vec<char> = input.chars().collect();
    let mut output: Vec<u8> = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if characters[index] == '|'
            && let Some(close) = closing_pipe(&characters, index)
        {
            decode_hex_run(&characters[index + 1..close], &mut output);
            index = close + 1;
            continue;
        }
        output.push(latin1(characters[index])?);
        index += 1;
    }
    context.ensure_active()?;
    Ok(output)
}

/// Finds the pipe closing a run of at least two hex-or-space characters.
fn closing_pipe(characters: &[char], open: usize) -> Option<usize> {
    let mut cursor = open + 1;
    while cursor < characters.len() {
        let character = characters[cursor];
        if character == '|' {
            return (cursor - open - 1 >= 2).then_some(cursor);
        }
        if !character.is_ascii_hexdigit() && character != ' ' {
            return None;
        }
        cursor += 1;
    }
    None
}

/// Decodes a pipe-delimited run, appending its bytes to `output`.
///
/// A space is a delimiter rather than something to ignore, so the run is cut
/// into groups first and each group is read in two-digit pairs on its own. The
/// distinction shows up on an odd number of digits: `|3d3|` is one group and
/// yields `0x3d` then `0x03` — the stray nibble becomes a byte of its own
/// rather than combining with a digit across the gap. `|3d 3|` yields the same
/// two bytes by a different route, while `|3 d3|` yields `0x03` then `0xd3`.
///
/// Empty groups — from a leading, trailing, or doubled space — contribute
/// nothing, which is why `|  |` decodes to no bytes at all rather than failing.
fn decode_hex_run(run: &[char], output: &mut Vec<u8>) {
    for group in run.split(|character| *character == ' ') {
        for pair in group.chunks(2) {
            let mut value = 0u32;
            for digit in pair {
                // Every character here came through `closing_pipe`, which
                // admits only hex digits and spaces, and the split removed the
                // spaces. The default cannot be reached.
                value = value * 16 + digit.to_digit(16).unwrap_or(0);
            }
            output.push(u8::try_from(value).unwrap_or(0));
        }
    }
}

const NOT_LATIN1: &str = "encoding.hex_content.not_latin1";

fn latin1(character: char) -> Result<u8, OperationError> {
    u8::try_from(u32::from(character)).map_err(|_| failed(NOT_LATIN1))
}
