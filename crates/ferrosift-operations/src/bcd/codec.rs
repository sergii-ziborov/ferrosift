use alloc::string::String;
use alloc::vec::Vec;

use crate::jscompat::delim::is_js_whitespace;
use crate::jscompat::number::{self as jsint, JsInt};

/// The nibble each decimal digit takes, in one of the reference's encodings.
///
/// Seven tables, and only the first is the obvious one. Two of the names
/// differ from another only in punctuation, which is worth knowing before
/// naming anything after them.
pub(crate) fn scheme(name: &str) -> Option<[u8; 10]> {
    Some(match name {
        "8 4 2 1" => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        "7 4 2 1" => [0, 1, 2, 3, 4, 5, 6, 8, 9, 10],
        "4 2 2 1" => [0, 1, 4, 5, 8, 9, 12, 13, 14, 15],
        "2 4 2 1" => [0, 1, 2, 3, 4, 11, 12, 13, 14, 15],
        "8 4 -2 -1" => [0, 7, 6, 5, 4, 11, 10, 9, 8, 15],
        "Excess-3" => [3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        "IBM 8 4 2 1" => [10, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        _ => return None,
    })
}

/// The sign nibbles: credit for a positive value, debit for anything else.
const CREDIT: u8 = 12;
const DEBIT: u8 = 13;

/// One place in the encoding, or a character the reference had no digit for.
///
/// `None` is not an error here, because it is not one there. A character that
/// is not a digit indexes the table with not-a-number and yields `undefined`,
/// which JavaScript then carries into the packing as a zero and into the two
/// binary renderings as a crash. `Infinity` reaches this: it passes both of
/// the operation's guards and encodes as eight of these.
type Place = Option<u8>;

/// Turns rendered digits into nibbles, with the sign nibble if one is wanted.
///
/// The leading zero is the rule worth stating. A sign nibble takes a place of
/// its own, so with an even number of packed digits it would sit alone in the
/// last byte and leave the reading unable to say whether the value ended in a
/// zero. The reference prepends a zero digit to make the count odd. It does
/// this only when packing — unpacked, every nibble has its own byte and the
/// question never arises.
pub(crate) fn nibbles(
    digits: &str,
    table: &[u8; 10],
    packed: bool,
    signed: bool,
    positive: bool,
) -> Vec<Place> {
    let mut places: Vec<Place> = digits
        .chars()
        .map(|character| character.to_digit(10).map(|digit| table[digit as usize]))
        .collect();

    if signed {
        if packed && digits.chars().count().is_multiple_of(2) {
            places.insert(0, Some(table[0]));
        }
        places.push(Some(if positive { CREDIT } else { DEBIT }));
    }
    places
}

/// Packs two nibbles to a byte, high first, padding the last with zero.
pub(crate) fn pack(places: &[Place]) -> Vec<Place> {
    let mut bytes = Vec::with_capacity(places.len().div_ceil(2));
    let mut encoded = 0_u8;
    let mut low = false;
    for place in places {
        // A place with no digit contributes nothing, in either half: the
        // reference shifts `undefined` and exclusive-ors zero.
        let value = place.unwrap_or(0);
        encoded ^= if low { value } else { value << 4 };
        if low {
            bytes.push(Some(encoded));
            encoded = 0;
        }
        low = !low;
    }
    if low {
        bytes.push(Some(encoded));
    }
    bytes
}

/// Gives each nibble its own byte, with a zero high half.
///
/// The reference builds this *after* taking the bytes, so the two differ: the
/// bytes are the nibbles themselves and this is the nibbles interleaved with
/// zeros. Both are then available to the renderer, and which one it uses
/// depends on the format.
pub(crate) fn spread(places: &[Place]) -> Vec<Place> {
    let mut spread = Vec::with_capacity(places.len() * 2);
    for place in places {
        spread.push(Some(0));
        spread.push(*place);
    }
    spread
}

/// Renders values as fixed-width binary, joined by spaces.
///
/// `None` has no rendering: the reference calls a method on `undefined` and
/// throws. That is why this returns an option rather than substituting a zero,
/// which would be the tempting thing and would disagree.
pub(crate) fn binary(values: &[Place], width: usize) -> Option<String> {
    let mut output = String::new();
    for value in values {
        let value = (*value)?;
        if !output.is_empty() {
            output.push(' ');
        }
        for bit in (0..width).rev() {
            output.push(if value >> bit & 1 == 1 { '1' } else { '0' });
        }
    }
    Some(output)
}

/// Renders bytes as the characters the reference hands to its dish.
///
/// `None` becomes a zero byte here rather than failing, because the reference
/// converts with `String.fromCharCode`, and that reads `undefined` as zero.
pub(crate) fn raw(values: &[Place]) -> String {
    values
        .iter()
        .map(|value| char::from(value.unwrap_or(0)))
        .collect()
}

/// Reads nibbles from text written as binary, four characters at a time.
///
/// A trailing group shorter than four is read as what it holds, so eleven
/// characters are three nibbles and the last is worth one rather than eight.
/// Anything unreadable is not-a-number, which the caller refuses.
pub(crate) fn read_binary(input: &str) -> Vec<Option<u8>> {
    let stripped: String = input
        .chars()
        .filter(|character| !is_js_whitespace(*character))
        .collect();
    let characters: Vec<char> = stripped.chars().collect();
    characters
        .chunks(4)
        .map(|chunk| {
            let text: String = chunk.iter().collect();
            match jsint::parse(&text, 2) {
                JsInt::Value(value) => u8::try_from(value).ok(),
                JsInt::Nan => None,
            }
        })
        .collect()
}

/// Reads nibbles from raw bytes, high half first.
pub(crate) fn read_raw(bytes: &[u8]) -> Vec<Option<u8>> {
    let mut nibbles = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        nibbles.push(Some(byte >> 4));
        nibbles.push(Some(byte & 15));
    }
    nibbles
}

/// Discards the high nibble of each byte, by the reference's own rule.
///
/// Written to match a loop that removes an element and then advances past the
/// next one, which is usually a bug and is the intended behaviour here: it
/// keeps the second of every pair. The tail of an odd-length run is dropped
/// rather than kept, which a tidier implementation would get wrong -- three
/// nibbles come back as one, not two.
pub(crate) fn discard_high(nibbles: &[Option<u8>]) -> Vec<Option<u8>> {
    let mut kept: Vec<Option<u8>> = nibbles.to_vec();
    let mut at = 0;
    while at < kept.len() {
        kept.remove(at);
        at += 1;
    }
    kept
}

/// The digit a nibble stands for in this encoding, if it stands for one.
pub(crate) fn digit_of(nibble: u8, table: &[u8; 10]) -> Option<u8> {
    table
        .iter()
        .position(|value| *value == nibble)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// Whether a sign nibble means the value is negative.
///
/// Two nibbles do, and one of them belongs to no encoding above: eleven is the
/// minus of a scheme this operation does not offer, and the reference accepts
/// it anyway.
pub(crate) const fn is_negative_sign(nibble: u8) -> bool {
    nibble == DEBIT || nibble == 11
}
