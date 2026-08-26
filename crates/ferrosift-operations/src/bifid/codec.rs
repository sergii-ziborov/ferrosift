use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_KEY: &str = "cipher.bifid.invalid_key";

/// The 25-letter square alphabet: J is folded onto I to fit five by five.
const ALPHA: &str = "ABCDEFGHIKLMNOPQRSTUVWXYZ";

/// What each input character contributed.
enum Slot {
    /// A letter that was upper case in the input.
    Upper,
    /// A letter that was lower case in the input.
    Lower,
    /// Anything else, carried through untouched.
    Literal(char),
}

/// Builds the 5×5 Polybius square for a keyword.
///
/// The keyword's letters come first, in order, each kept only the first time
/// it appears; the rest of the alphabet follows on the same rule. A keyword
/// that repeats a letter is therefore not an error — the repeat simply does
/// not move anything.
fn polybius_square(keyword: &str) -> Vec<char> {
    let mut square: Vec<char> = Vec::with_capacity(25);
    for character in keyword.chars().chain(ALPHA.chars()) {
        if !square.contains(&character) {
            square.push(character);
        }
    }
    square
}

/// Normalises the keyword the way the reference does.
///
/// `String.replace` with a string pattern replaces the *first* match only, so
/// a keyword of `JAJA` becomes `IAJA` rather than `IAIA` — and the surviving
/// `J` then fails the letters-only check, because the square's alphabet has no
/// J in it. That is the reference's behaviour, not a rounding of it.
fn normalise_keyword(keyword: &str) -> Result<String, OperationError> {
    let upper: String = keyword.chars().flat_map(char::to_uppercase).collect();
    let folded = replace_first(&upper, 'J', 'I');
    if !folded.is_empty() && !folded.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(failed(INVALID_KEY));
    }
    Ok(folded)
}

/// Replaces the first occurrence of one character, as `String.replace` does.
fn replace_first(input: &str, from: char, to: char) -> String {
    match input.find(from) {
        Some(at) => {
            let mut output = String::with_capacity(input.len());
            output.push_str(&input[..at]);
            output.push(to);
            output.push_str(&input[at + from.len_utf8()..]);
            output
        }
        None => String::from(input),
    }
}

/// Splits the input into square coordinates and a record of what was where.
///
/// Returns the row of each letter, the column of each letter, and one slot per
/// input character so the original casing and punctuation can be put back.
fn scan(input: &str, square: &[char]) -> (Vec<usize>, Vec<usize>, Vec<Slot>) {
    let mut rows = Vec::new();
    let mut columns = Vec::new();
    let mut structure = Vec::new();
    for character in input.chars() {
        let upper = character.to_uppercase().next().unwrap_or(character);
        match square.iter().position(|c| *c == upper) {
            Some(index) if ALPHA.contains(upper) => {
                rows.push(index / 5);
                columns.push(index % 5);
                structure.push(if ALPHA.contains(character) {
                    Slot::Upper
                } else {
                    Slot::Lower
                });
            }
            _ => structure.push(Slot::Literal(character)),
        }
    }
    (rows, columns, structure)
}

/// Encodes text with the Bifid cipher.
///
/// Every row is written out, then every column, and the joined sequence is
/// read back two digits at a time. That fractionation is the whole cipher: a
/// letter's row and its column end up in different halves of the message, so
/// each output letter depends on two input letters that may be far apart.
pub(super) fn encode(
    input: &str,
    keyword: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let keyword = normalise_keyword(keyword)?;
    let square = polybius_square(&keyword);
    let folded = replace_first(input, 'J', 'I');
    let (rows, columns, structure) = scan(&folded, &square);

    let mut stream: Vec<usize> = rows;
    stream.extend(columns);

    let mut output = String::new();
    let mut count: usize = 0;
    for slot in &structure {
        if count.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        match slot {
            Slot::Literal(character) => output.push(*character),
            Slot::Upper | Slot::Lower => {
                let row = stream.get(count * 2).copied().unwrap_or(0);
                let column = stream.get(count * 2 + 1).copied().unwrap_or(0);
                push_letter(
                    &mut output,
                    &square,
                    row,
                    column,
                    matches!(slot, Slot::Upper),
                );
                count += 1;
            }
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// Decodes Bifid-ciphered text.
///
/// The coordinate stream is built interleaved — row then column per letter —
/// and read back split: the first half supplies rows, the second columns. That
/// is the exact inverse of the encoder's write-all-rows-then-all-columns.
pub(super) fn decode(
    input: &str,
    keyword: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let keyword = normalise_keyword(keyword)?;
    let square = polybius_square(&keyword);
    let folded = replace_first(input, 'J', 'I');
    let (rows, columns, structure) = scan(&folded, &square);

    let mut stream: Vec<usize> = Vec::with_capacity(rows.len() * 2);
    for (row, column) in rows.iter().zip(&columns) {
        stream.push(*row);
        stream.push(*column);
    }
    let half = stream.len() / 2;

    let mut output = String::new();
    let mut count: usize = 0;
    for slot in &structure {
        if count.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        match slot {
            Slot::Literal(character) => output.push(*character),
            Slot::Upper | Slot::Lower => {
                let row = stream.get(count).copied().unwrap_or(0);
                let column = stream.get(count + half).copied().unwrap_or(0);
                push_letter(
                    &mut output,
                    &square,
                    row,
                    column,
                    matches!(slot, Slot::Upper),
                );
                count += 1;
            }
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// Appends the square's letter at one coordinate, in the recorded case.
fn push_letter(output: &mut String, square: &[char], row: usize, column: usize, upper: bool) {
    let letter = square.get(row * 5 + column).copied().unwrap_or('?');
    if upper {
        output.push(letter);
    } else {
        output.extend(letter.to_lowercase());
    }
}
