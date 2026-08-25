use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// Which parity the encoder targets.
#[derive(Clone, Copy)]
pub(super) enum Parity {
    Even,
    Odd,
}

/// Which end of the run the bit sits on.
#[derive(Clone, Copy)]
pub(super) enum Position {
    Start,
    End,
}

/// Encode or decode.
#[derive(Clone, Copy)]
pub(super) enum Direction {
    Encode,
    Decode,
}

pub(super) fn parity(value: &str) -> Result<Parity, OperationError> {
    match value {
        "Even Parity" => Ok(Parity::Even),
        "Odd Parity" => Ok(Parity::Odd),
        _ => Err(failed("logic.parity.invalid_mode")),
    }
}

pub(super) fn position(value: &str) -> Result<Position, OperationError> {
    match value {
        "Start" => Ok(Position::Start),
        "End" => Ok(Position::End),
        _ => Err(failed("logic.parity.invalid_position")),
    }
}

pub(super) fn direction(value: &str) -> Result<Direction, OperationError> {
    match value {
        "Encode" => Ok(Direction::Encode),
        "Decode" => Ok(Direction::Decode),
        _ => Err(failed("logic.parity.invalid_direction")),
    }
}

/// Adds or removes a parity bit, optionally per delimited field.
///
/// An empty input is returned untouched before anything else, matching the
/// reference's first line — with a delimiter set, an empty input would
/// otherwise split into one empty field and gain a bit.
pub(super) fn parity_bit(
    input: &str,
    parity: Parity,
    position: Position,
    direction: Direction,
    delimiter: &str,
) -> Result<String, OperationError> {
    if input.is_empty() {
        return Ok(String::new());
    }
    if delimiter.is_empty() {
        return apply(input, parity, position, direction, delimiter);
    }
    let mut fields: Vec<String> = Vec::new();
    for field in input.split(delimiter) {
        fields.push(apply(field, parity, position, direction, delimiter)?);
    }
    Ok(fields.join(delimiter))
}

fn apply(
    input: &str,
    parity: Parity,
    position: Position,
    direction: Direction,
    delimiter: &str,
) -> Result<String, OperationError> {
    match direction {
        Direction::Encode => encode(input, parity, position, delimiter),
        Direction::Decode => Ok(decode(input, position)),
    }
}

fn encode(
    input: &str,
    parity: Parity,
    position: Position,
    delimiter: &str,
) -> Result<String, OperationError> {
    let mut ones = 0usize;
    for character in input.chars() {
        if character == '1' {
            ones += 1;
        } else if character != '0' && character != ' ' && !delimiter.starts_with(character) {
            // The reference compares the character against the delimiter
            // argument itself, so a multi-character delimiter only excuses its
            // own first character. Reproduced rather than corrected.
            return Err(failed("logic.parity.unexpected_character"));
        }
    }
    // Even parity wants an even count of ones, so the bit is zero when the
    // count already matches; odd parity flips which remainder that is.
    let target = match parity {
        Parity::Even => 0,
        Parity::Odd => 1,
    };
    let bit = if ones % 2 == target { '0' } else { '1' };

    let mut output = String::with_capacity(input.len() + 1);
    match position {
        Position::End => {
            output.push_str(input);
            output.push(bit);
        }
        Position::Start => {
            output.push(bit);
            output.push_str(input);
        }
    }
    Ok(output)
}

/// Drops the parity bit without checking it, as the reference does.
fn decode(input: &str, position: Position) -> String {
    let characters: Vec<char> = input.chars().collect();
    if characters.is_empty() {
        return String::new();
    }
    match position {
        Position::End => characters[..characters.len() - 1].iter().collect(),
        Position::Start => characters[1..].iter().collect(),
    }
}

/// Which cases the MAC formatter emits.
#[derive(Clone, Copy)]
pub(super) enum OutputCase {
    Both,
    Upper,
    Lower,
}

pub(super) fn output_case(value: &str) -> Result<OutputCase, OperationError> {
    match value {
        "Both" => Ok(OutputCase::Both),
        "Upper only" => Ok(OutputCase::Upper),
        "Lower only" => Ok(OutputCase::Lower),
        _ => Err(failed("network.mac.invalid_case")),
    }
}

/// Which delimiter styles to emit.
///
/// Five independent toggles because the reference has five, and they compose:
/// a caller can ask for dashes and Cisco style at once. Collapsing them into
/// an enum would change the operation's contract, not just its shape.
#[derive(Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per reference argument; they are independent, not a state machine"
)]
pub(super) struct MacStyles {
    pub none: bool,
    pub dash: bool,
    pub colon: bool,
    pub cisco: bool,
    pub ipv6: bool,
}

/// Rewrites each MAC address in every requested style.
///
/// Input is split on commas and whitespace, and every group is followed by an
/// empty line — including the last, so the output ends with a newline. That
/// trailing blank is the reference's own shape and is what a corpus case would
/// catch if it were dropped.
pub(super) fn format_macs(input: &str, case: OutputCase, styles: MacStyles) -> String {
    if input.is_empty() {
        return String::new();
    }
    let lowered = input.to_lowercase();
    let mut output: Vec<String> = Vec::new();

    for mac in lowered.split([',', ' ', '\t', '\r', '\n']) {
        if mac.is_empty() {
            // `split` on a character class collapses nothing, but the
            // reference's regex `[,\s\r\n]+` does — consecutive separators
            // yield one field there, so empties are skipped here to match.
            continue;
        }
        let clean: String = mac
            .chars()
            .filter(|value| !matches!(value, ':' | '.' | '-'))
            .collect();
        let hyphen = group(&clean, 2, '-');
        let colon = group(&clean, 2, ':');
        let cisco = group(&clean, 4, '.');
        let ipv6 = interface_id(&clean);

        let mut push = |value: &str| match case {
            OutputCase::Lower => output.push(value.to_string()),
            OutputCase::Upper => output.push(value.to_uppercase()),
            OutputCase::Both => {
                output.push(value.to_string());
                output.push(value.to_uppercase());
            }
        };
        if styles.none {
            push(&clean);
        }
        if styles.dash {
            push(&hyphen);
        }
        if styles.colon {
            push(&colon);
        }
        if styles.cisco {
            push(&cisco);
        }
        if styles.ipv6 {
            push(&ipv6);
        }
        // Empty line delimiting each group, the reference's own separator.
        output.push(String::new());
    }
    output.join("\n")
}

/// Inserts `separator` after every `size` characters except the last group.
fn group(value: &str, size: usize, separator: char) -> String {
    let characters: Vec<char> = value.chars().collect();
    let mut output = String::with_capacity(characters.len() + characters.len() / size);
    for (index, character) in characters.iter().enumerate() {
        output.push(*character);
        // `(.{n}(?=.))` only inserts where another character follows.
        if (index + 1) % size == 0 && index + 1 < characters.len() {
            output.push(separator);
        }
    }
    output
}

/// The EUI-64 interface identifier: `fffe` inserted, then bit 1 of byte 0
/// flipped.
fn interface_id(clean: &str) -> String {
    let characters: Vec<char> = clean.chars().collect();
    let mut expanded: String = characters.iter().take(6).collect();
    expanded.push_str("fffe");
    expanded.extend(characters.iter().skip(6));

    let grouped = group(&expanded, 4, ':');
    // `parseInt(slice(0, 2), 16) ^ 2`, re-emitted as two lower-case hex
    // digits.
    //
    // `parseInt` takes the longest valid hex prefix and gives NaN when there
    // is none — and JavaScript's bitwise operators coerce NaN to zero, so a
    // pair like "he" yields `0 ^ 2` and the octet becomes "02". That only
    // happens for input that is not a MAC address at all, which the reference
    // reformats anyway rather than rejecting, so it is reachable and pinned.
    let parsed = match crate::jsint::parse(grouped.get(..2).unwrap_or(""), 16) {
        crate::jsint::JsInt::Nan => 0,
        crate::jsint::JsInt::Value(value) => value,
    };
    let flipped = u8::try_from(parsed & 0xff).unwrap_or(0) ^ 2;
    let mut output = String::with_capacity(grouped.len());
    output.push(hex_digit(flipped >> 4));
    output.push(hex_digit(flipped & 0x0f));
    output.push_str(grouped.get(2..).unwrap_or(""));
    output
}

const fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'a' + value - 10) as char,
    }
}
