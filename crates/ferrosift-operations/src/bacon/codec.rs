use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_ALPHABET: &str = "cipher.bacon.invalid_alphabet";
const INVALID_TRANSLATION: &str = "cipher.bacon.invalid_translation";

/// The 24-letter alphabet, which folds I with J and U with V.
const STANDARD: &str = "ABCDEFGHIKLMNOPQRSTUWXYZ";
/// The 26-letter alphabet, which folds nothing.
const COMPLETE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Code assigned to each of A–Z under the folding alphabet.
///
/// I and J both take 8, U and V both take 19, so the 26 letters map onto 24
/// codes. Written out rather than computed: the folding is a fact about the
/// historical cipher, not a rule with an off-by-one to derive.
const STANDARD_CODES: [u8; 26] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 23,
];

/// Which alphabet a recipe selected.
#[derive(Clone, Copy)]
pub(super) enum Alphabet {
    Standard,
    Complete,
}

impl Alphabet {
    pub(super) fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "Standard (I=J and U=V)" => Ok(Self::Standard),
            "Complete" => Ok(Self::Complete),
            _ => Err(failed(INVALID_ALPHABET)),
        }
    }

    fn letters(self) -> &'static str {
        match self {
            Self::Standard => STANDARD,
            Self::Complete => COMPLETE,
        }
    }

    fn code(self, index: u8) -> u8 {
        match self {
            Self::Standard => STANDARD_CODES[usize::from(index)],
            Self::Complete => index,
        }
    }
}

/// How the two symbols of the code are written down.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Translation {
    ZeroOne,
    AB,
    Case,
    FirstLetter,
}

impl Translation {
    pub(super) fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "0/1" => Ok(Self::ZeroOne),
            "A/B" => Ok(Self::AB),
            "Case" => Ok(Self::Case),
            "A-M/N-Z first letter" => Ok(Self::FirstLetter),
            _ => Err(failed(INVALID_TRANSLATION)),
        }
    }

    /// Encoding writes only the two symbolic forms; the steganographic ones
    /// need a carrier text the operation is not given.
    pub(super) fn encodable(self) -> Result<Self, OperationError> {
        match self {
            Self::ZeroOne | Self::AB => Ok(self),
            Self::Case | Self::FirstLetter => Err(failed(INVALID_TRANSLATION)),
        }
    }
}

/// Encodes text as a Bacon cipher.
pub(super) fn encode(
    input: &str,
    alphabet: Alphabet,
    translation: Translation,
    keep: bool,
    invert: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::new();
    for (index, character) in input.chars().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        match upper_index(character) {
            Some(letter) => {
                let code = alphabet.code(letter);
                for position in (0..5).rev() {
                    output.push(if (code >> position) & 1 == 1 {
                        '1'
                    } else {
                        '0'
                    });
                }
            }
            None => output.push(character),
        }
    }

    // Inversion runs before anything is discarded, so a `0` or `1` that was
    // literal input text is flipped along with the code the cipher produced.
    if invert {
        output = swap_zero_one(&output);
    }
    if !keep {
        let bits: Vec<char> = output.chars().filter(|c| *c == '0' || *c == '1').collect();
        let groups: Vec<String> = bits.chunks_exact(5).map(|g| g.iter().collect()).collect();
        output = groups.join(" ");
    }
    if translation == Translation::AB {
        output = output
            .chars()
            .map(|c| match c {
                '0' => 'A',
                '1' => 'B',
                other => other,
            })
            .collect();
    }
    context.ensure_active()?;
    Ok(output)
}

/// The A–Z index of a character after upper-casing it, if it has one.
///
/// The reference upper-cases the character and reads `charCodeAt(0)`, so a
/// character whose upper case is more than one character contributes only its
/// first — `ß` becomes `SS` and is read as `S`. Taking the first of Rust's
/// upper-case iterator is the same rule.
fn upper_index(character: char) -> Option<u8> {
    let upper = character.to_uppercase().next()?;
    let code = u32::from(upper);
    (u32::from('A')..=u32::from('Z'))
        .contains(&code)
        .then(|| u8::try_from(code - u32::from('A')).unwrap_or(0))
}

/// Decodes a Bacon cipher back to letters.
pub(super) fn decode(
    input: &str,
    alphabet: Alphabet,
    translation: Translation,
    invert: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut bits = match translation {
        Translation::ZeroOne => keep_only(input, |c| c == '0' || c == '1'),
        Translation::AB => keep_only(input, |c| matches!(c, 'A' | 'B' | 'a' | 'b'))
            .chars()
            .map(|c| if c == 'A' || c == 'a' { '0' } else { '1' })
            .collect(),
        Translation::Case => keep_only(input, |c: char| c.is_ascii_alphabetic())
            .chars()
            .map(|c| if c.is_ascii_uppercase() { '1' } else { '0' })
            .collect(),
        Translation::FirstLetter => first_letters(&strip_undefined(input)),
    };

    if invert {
        bits = swap_zero_one(&bits);
    }

    let symbols: Vec<char> = bits.chars().collect();
    let letters = alphabet.letters().as_bytes();
    let mut output = String::new();
    for (index, group) in symbols.chunks_exact(5).enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let mut value = 0usize;
        for symbol in group {
            value = value * 2 + usize::from(*symbol == '1');
        }
        output.push(match letters.get(value) {
            Some(letter) => char::from(*letter),
            None => '?',
        });
    }
    context.ensure_active()?;
    Ok(output)
}

/// Removes the first literal `undefined` from the input.
///
/// This is not a joke at the reference's expense; it is what the reference
/// does. The table of "characters to strip" has no entry for this translation,
/// so `String.replace` is handed `undefined`, coerces it to the *string*
/// `"undefined"`, and removes the first occurrence of that text. Nothing else
/// is stripped, which is why the first-letter mode alone tolerates punctuation.
///
/// Reproducing it costs three lines. Leaving it out would mean any input
/// containing the word decodes differently here than there, which is exactly
/// the kind of silent divergence the corpus exists to prevent.
fn strip_undefined(input: &str) -> String {
    match input.find("undefined") {
        Some(at) => {
            let mut output = String::from(&input[..at]);
            output.push_str(&input[at + "undefined".len()..]);
            output
        }
        None => String::from(input),
    }
}

/// Reads one bit from the first letter of each whitespace-separated word.
///
/// A word beginning at or after `N` is a one, anything earlier is a zero.
/// Empty words — from leading or repeated whitespace — contribute nothing.
fn first_letters(input: &str) -> String {
    input
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .map(|first| {
            let upper = first.to_uppercase().next().unwrap_or(first);
            if u32::from(upper) >= u32::from('N') {
                '1'
            } else {
                '0'
            }
        })
        .collect()
}

fn keep_only(input: &str, keep: impl Fn(char) -> bool) -> String {
    input.chars().filter(|c| keep(*c)).collect()
}

fn swap_zero_one(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '0' => '1',
            '1' => '0',
            other => other,
        })
        .collect()
}
