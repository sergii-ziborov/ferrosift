//! Cetacean, the NATO spelling alphabet, and leet speak.

use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

use super::letters::{from_units, units};

const INVALID_DIRECTION: &str = "text.leet.invalid_direction";
/// Encodes each code unit as sixteen `e`/`E` bits, leaving spaces alone.
pub(super) fn cetacean_encode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::new();
    for (position, unit) in units(input).iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if *unit == u16::from(b' ') {
            output.push(' ');
            continue;
        }
        for bit in (0..16).rev() {
            output.push(if (unit >> bit) & 1 == 1 { 'e' } else { 'E' });
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// Reads sixteen-bit groups back, expanding a space to the bits of `0x20`.
///
/// The reference iterates code points here rather than code units, so an
/// astral character contributes one bit, not two.
pub(super) fn cetacean_decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut bits: Vec<u16> = Vec::new();
    for (position, character) in input.chars().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if character == ' ' {
            bits.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
        } else {
            bits.push(u16::from(character == 'e'));
        }
    }
    let mut output: Vec<u16> = Vec::new();
    for group in bits.chunks(16) {
        let mut value: u16 = 0;
        for bit in group {
            value = (value << 1) | bit;
        }
        output.push(value);
    }
    context.ensure_active()?;
    Ok(from_units(&output))
}

/// The NATO word for each letter, digit, and the three punctuation marks the
/// reference covers. Each carries its own trailing space, as the reference's
/// lookup table does.
const NATO: [(char, &str); 39] = [
    ('A', "Alfa "),
    ('B', "Bravo "),
    ('C', "Charlie "),
    ('D', "Delta "),
    ('E', "Echo "),
    ('F', "Foxtrot "),
    ('G', "Golf "),
    ('H', "Hotel "),
    ('I', "India "),
    ('J', "Juliett "),
    ('K', "Kilo "),
    ('L', "Lima "),
    ('M', "Mike "),
    ('N', "November "),
    ('O', "Oscar "),
    ('P', "Papa "),
    ('Q', "Quebec "),
    ('R', "Romeo "),
    ('S', "Sierra "),
    ('T', "Tango "),
    ('U', "Uniform "),
    ('V', "Victor "),
    ('W', "Whiskey "),
    ('X', "X-ray "),
    ('Y', "Yankee "),
    ('Z', "Zulu "),
    ('0', "Zero "),
    ('1', "One "),
    ('2', "Two "),
    ('3', "Three "),
    ('4', "Four "),
    ('5', "Five "),
    ('6', "Six "),
    ('7', "Seven "),
    ('8', "Eight "),
    ('9', "Nine "),
    (',', "Comma "),
    ('/', "Fraction bar "),
    ('.', "Full stop "),
];

/// Replaces every letter, digit, comma, slash, and full stop with its word.
pub(super) fn to_nato(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::new();
    for (position, character) in input.chars().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let upper = character.to_ascii_uppercase();
        match NATO.iter().find(|(key, _)| *key == upper) {
            Some((_, word)) if character.is_ascii() => output.push_str(word),
            _ => output.push(character),
        }
    }
    context.ensure_active()?;
    Ok(output)
}

const TO_LEET: [(char, char); 6] = [
    ('a', '4'),
    ('e', '3'),
    ('i', '1'),
    ('o', '0'),
    ('s', '5'),
    ('t', '7'),
];

/// Converts to or from leet speak.
///
/// The reverse direction's character class includes `6`, which the reference's
/// map has no entry for, so a `6` passes through unchanged. Upper-case letters
/// match the class through its `i` flag but miss the lower-case-keyed map, so
/// they pass through too — `ABC` decodes to `ABC`, not `abc`.
pub(super) fn leet(
    input: &str,
    direction: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let encoding = match direction {
        "To Leet Speak" => true,
        "From Leet Speak" => false,
        _ => return Err(failed(INVALID_DIRECTION)),
    };
    let mut output = String::new();
    for (position, character) in input.chars().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        output.push(if encoding {
            leet_encode_char(character)
        } else {
            leet_decode_char(character)
        });
    }
    context.ensure_active()?;
    Ok(output)
}

fn leet_encode_char(character: char) -> char {
    if !character.is_ascii_alphabetic() {
        return character;
    }
    let lower = character.to_ascii_lowercase();
    TO_LEET
        .iter()
        .find(|(letter, _)| *letter == lower)
        .map_or(character, |(_, leet)| *leet)
}

fn leet_decode_char(character: char) -> char {
    TO_LEET
        .iter()
        .find(|(_, leet)| *leet == character)
        .map_or(character, |(letter, _)| *letter)
}
