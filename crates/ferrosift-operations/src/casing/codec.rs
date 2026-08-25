use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// Scope for capitalisation.
#[derive(Clone, Copy)]
pub(super) enum Scope {
    All,
    Word,
    Sentence,
    Paragraph,
}

pub(super) fn scope(value: &str) -> Result<Scope, OperationError> {
    match value {
        "All" => Ok(Scope::All),
        "Word" => Ok(Scope::Word),
        "Sentence" => Ok(Scope::Sentence),
        "Paragraph" => Ok(Scope::Paragraph),
        _ => Err(failed("text.case.invalid_scope")),
    }
}

/// The reference matches word starts with `\w`, which is ASCII in JavaScript
/// regardless of the Unicode flag: letters, digits, and underscore. Using
/// Rust's `is_alphanumeric` here would additionally capitalise scripts the
/// reference leaves alone, so the ASCII definition is what gets ported.
fn is_word_byte(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

/// Upper-cases the first word character of each region the scope selects.
///
/// The reference expresses this as three regular expressions. Each one finds a
/// word character that follows a region boundary — the start of input, a full
/// stop, or a newline — with optional whitespace between. Walking the string
/// once decides the same positions without a regex engine.
pub(super) fn capitalise(input: &str, scope: Scope) -> String {
    if matches!(scope, Scope::All) {
        return input.to_uppercase();
    }

    let mut output = String::with_capacity(input.len());
    // Armed means the next word character is the one the pattern captures.
    // It starts true because every pattern offers `^` as an alternative to its
    // boundary character.
    let mut armed = true;

    for value in input.chars() {
        if is_word_byte(value) {
            if armed {
                output.extend(value.to_uppercase());
            } else {
                output.push(value);
            }
            // `\w` consumed: the next word character is mid-word, not a start.
            armed = false;
            continue;
        }

        output.push(value);
        armed = match scope {
            // `\b\w`: any non-word character opens a word boundary.
            Scope::Word => true,
            // `(?:\.|^)\s*\b\w`: only the boundary character arms, and the
            // `\s*` lets that survive across whitespace but nothing else.
            Scope::Sentence => value == '.' || (armed && value.is_whitespace()),
            Scope::Paragraph => value == '\n' || (armed && value.is_whitespace()),
            Scope::All => false,
        };
    }
    output
}

/// Swaps the case of every character.
///
/// The reference asks whether a character equals its own upper-casing and
/// lower-cases it if so. That is not the same as "is lower-case": a character
/// with no case, and one whose upper-casing is longer than itself, both take
/// the other branch. Comparing the mapped string rather than a single
/// character is what keeps those cases agreeing.
pub(super) fn swap_case(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for value in input.chars() {
        let upper: String = value.to_uppercase().collect();
        if upper.chars().eq(core::iter::once(value)) {
            output.extend(value.to_lowercase());
        } else {
            output.push_str(&upper);
        }
    }
    output
}

/// Alternates case across letters, leaving everything else alone.
///
/// The first letter is lower-cased, because the reference starts with its
/// "previous was capital" flag already set.
pub(super) fn alternating(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut previous_upper = true;
    for value in input.chars() {
        if value.is_alphabetic() {
            if previous_upper {
                output.extend(value.to_lowercase());
            } else {
                output.extend(value.to_uppercase());
            }
            previous_upper = !previous_upper;
        } else {
            output.push(value);
        }
    }
    output
}

/// Every combination of upper and lower case, one per line.
///
/// This is exponential in the input length by definition — the reference
/// builds `1 << length` lines — so a caller who pastes a sentence into it asks
/// for more output than any machine has. The reference discovers that by
/// exhausting memory. Here the size is computed first and refused against the
/// budget, which is the difference between a limit and a crash.
pub(super) fn all_casings(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let characters: Vec<char> = input.chars().flat_map(char::to_lowercase).collect();
    let length = characters.len();
    if length == 0 {
        return Ok(String::new());
    }

    // `1 << length` overflows past 63, and the budget check below would refuse
    // anything near it long before, so the count is computed defensively.
    let combinations = 1u64
        .checked_shl(u32::try_from(length).map_err(|_| OperationError::OutputLimitExceeded)?)
        .ok_or(OperationError::OutputLimitExceeded)?;
    let line = input.len() + 1;
    let capacity = combinations
        .checked_mul(u64::try_from(line).map_err(|_| OperationError::OutputLimitExceeded)?)
        .ok_or(OperationError::OutputLimitExceeded)?;
    if capacity > context.budget().max_output_bytes {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut output = String::with_capacity(
        usize::try_from(capacity).map_err(|_| OperationError::OutputLimitExceeded)?,
    );
    for mask in 0..combinations {
        if mask % 1024 == 0 {
            context.ensure_active()?;
        }
        for (index, value) in characters.iter().enumerate() {
            if (mask >> index) & 1 == 1 {
                output.extend(value.to_uppercase());
            } else {
                output.push(*value);
            }
        }
        output.push('\n');
    }
    // The reference drops the trailing newline with a slice.
    output.pop();
    context.ensure_active()?;
    Ok(output)
}

/// Lower-cases the whole input.
pub(super) fn lower(input: &str) -> String {
    input.to_lowercase()
}
