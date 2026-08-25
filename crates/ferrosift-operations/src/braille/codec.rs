use alloc::string::String;
use alloc::vec::Vec;

/// The reference's ASCII half of the braille table, in its own order.
///
/// Position matters: this is an index-for-index pairing with the dot patterns
/// below, not an alphabet. The space at position zero is the blank cell, and
/// the punctuation ordering is North American Braille ASCII rather than
/// anything derivable.
const BRAILLE_ASCII: &str = " A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=";

/// Dot patterns U+2800 through U+283F, in the same order.
///
/// Generated rather than written out: the block is contiguous and the
/// reference's string is exactly the first sixty-four code points of it, so
/// spelling them out would add sixty-four chances to mistype and none to be
/// clearer.
fn dot6(index: usize) -> Option<char> {
    (index < 64).then(|| char::from_u32(0x2800 + u32::try_from(index).unwrap_or(0)))?
}

/// Transcribes ASCII into braille cells.
///
/// Input is upper-cased per character before lookup, so both cases map to the
/// same cell. Anything not in the table passes through unchanged.
pub(super) fn to_braille(input: &str) -> String {
    let table: Vec<char> = BRAILLE_ASCII.chars().collect();
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        // `c.toUpperCase()` on one character, then `indexOf` on the result.
        // A character whose upper-casing is longer than itself cannot match a
        // single-character table entry, so it falls through unchanged.
        let upper: String = character.to_uppercase().collect();
        let matched =
            single(&upper).and_then(|value| table.iter().position(|entry| *entry == value));
        match matched.and_then(dot6) {
            Some(cell) => output.push(cell),
            None => output.push(character),
        }
    }
    output
}

/// Transcribes braille cells back into ASCII.
pub(super) fn from_braille(input: &str) -> String {
    let table: Vec<char> = BRAILLE_ASCII.chars().collect();
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        let index = (character as u32)
            .checked_sub(0x2800)
            .and_then(|offset| usize::try_from(offset).ok())
            .filter(|offset| *offset < 64);
        match index.and_then(|offset| table.get(offset)) {
            Some(letter) => output.push(*letter),
            None => output.push(character),
        }
    }
    output
}

fn single(value: &str) -> Option<char> {
    let mut characters = value.chars();
    let first = characters.next()?;
    characters.next().is_none().then_some(first)
}

/// Appends combining marks after every character.
///
/// The reference works on a byte array and pushes the UTF-8 encoding of each
/// mark after each *byte*, which for multi-byte input puts marks inside a
/// character. Operating on characters here would be tidier and would not match,
/// so the byte behaviour is what is implemented.
///
/// Strikethrough is applied before underline, which is the order the marks end
/// up in.
pub(super) fn unicode_text_format(input: &[u8], underline: bool, strikethrough: bool) -> Vec<u8> {
    // U+0336 combining long stroke overlay, U+0332 combining low line.
    const STRIKE: &[u8] = &[0xcc, 0xb6];
    const UNDER: &[u8] = &[0xcc, 0xb2];

    let mut marks: Vec<&[u8]> = Vec::new();
    if strikethrough {
        marks.push(STRIKE);
    }
    if underline {
        marks.push(UNDER);
    }

    let mut output = Vec::with_capacity(input.len() * (1 + marks.len() * 2));
    for byte in input {
        output.push(*byte);
        for mark in &marks {
            output.extend_from_slice(mark);
        }
    }
    output
}
