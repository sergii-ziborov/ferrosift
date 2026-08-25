use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// What the caret decoder has seen but not yet resolved.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    Nothing,
    M,
    MDash,
    MDashCaret,
    Caret,
}

/// Decodes `^X` and `M-X` sequences into the bytes they name.
///
/// An incomplete sequence at the end of the input is dropped rather than
/// emitted. The reference's loop simply ends with its `prev` still set and
/// never flushes it, so trailing `M`, `M-`, `M-^`, or `^` disappear. That
/// looks like an oversight and is reproduced anyway, because the bytes are
/// what is being matched.
///
/// # Errors
///
/// Returns an error when a code unit will not fit in a byte. The reference
/// pushes the raw `charCodeAt` value, which for anything above U+00FF is not
/// a byte at all, and then fails converting the result — it declines to
/// answer. Refusing here says that up front rather than inventing a value.
pub(super) fn caret_m_decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut bytes: Vec<u8> = Vec::with_capacity(input.len());
    let mut pending = Pending::Nothing;

    for (index, unit) in input.encode_utf16().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        let code = u32::from(unit);
        let byte = || -> Result<u8, OperationError> {
            u8::try_from(code).map_err(|_| failed("text.caret.not_a_byte"))
        };

        match pending {
            Pending::MDashCaret => {
                if code > 63 && code <= 95 {
                    bytes.push(byte()? + 64);
                } else if code == 63 {
                    bytes.push(255);
                } else {
                    bytes.extend_from_slice(&[77, 45, 94]);
                    bytes.push(byte()?);
                }
                pending = Pending::Nothing;
            }
            Pending::MDash => {
                if unit == u16::from(b'^') {
                    pending = Pending::MDashCaret;
                } else if (32..=126).contains(&code) {
                    bytes.push(byte()? + 128);
                    pending = Pending::Nothing;
                } else {
                    bytes.extend_from_slice(&[77, 45]);
                    bytes.push(byte()?);
                    pending = Pending::Nothing;
                }
            }
            Pending::M => {
                if unit == u16::from(b'-') {
                    pending = Pending::MDash;
                } else {
                    bytes.push(77);
                    bytes.push(byte()?);
                    pending = Pending::Nothing;
                }
            }
            Pending::Caret => {
                if code > 63 && code <= 126 {
                    bytes.push(byte()? - 64);
                } else if code == 63 {
                    bytes.push(127);
                } else {
                    bytes.push(94);
                    bytes.push(byte()?);
                }
                pending = Pending::Nothing;
            }
            Pending::Nothing => {
                if unit == u16::from(b'M') {
                    pending = Pending::M;
                } else if unit == u16::from(b'^') {
                    pending = Pending::Caret;
                } else {
                    bytes.push(byte()?);
                }
            }
        }
    }

    context.ensure_active()?;
    Ok(bytes)
}

/// Folds `[aA]`-style pairs back to a single letter.
///
/// The reference matches `\[[a-z]{2}\]` case-insensitively and keeps the first
/// letter only when both letters are the same one. A pair like `[ab]` is left
/// alone because it is a real character class, not a case fold.
pub(super) fn from_case_insensitive_regex(input: &str) -> String {
    let characters: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < characters.len() {
        if characters[index] == '['
            && index + 3 < characters.len()
            && characters[index + 3] == ']'
            && characters[index + 1].is_ascii_alphabetic()
            && characters[index + 2].is_ascii_alphabetic()
            && characters[index + 1].eq_ignore_ascii_case(&characters[index + 2])
        {
            output.push(characters[index + 1]);
            index += 4;
        } else {
            output.push(characters[index]);
            index += 1;
        }
    }
    output
}

/// Every subset of the delimited items, one per line.
///
/// The reference counts from zero to `2^n - 1`, treating each binary digit as
/// "keep this item", then sorts the joined results by length. That sort is
/// stable, so subsets of equal length keep the order the counting produced —
/// which is the only reason this output is reproducible at all.
///
/// Like every power set this is exponential, so the size is computed and
/// refused against the budget before anything is built.
pub(super) fn power_set(
    input: &str,
    delimiter: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    // The reference splits, then filters empty strings — twice, in two places.
    let items: Vec<&str> = if delimiter.is_empty() {
        // Splitting on an empty delimiter yields the characters themselves.
        input.split("").filter(|item| !item.is_empty()).collect()
    } else {
        input
            .split(delimiter)
            .filter(|item| !item.is_empty())
            .collect()
    };
    if items.is_empty() {
        return Ok(String::new());
    }

    let count = 1u64
        .checked_shl(u32::try_from(items.len()).map_err(|_| OperationError::OutputLimitExceeded)?)
        .ok_or(OperationError::OutputLimitExceeded)?;
    // Every subset is at most the whole input plus its newline.
    let bound = count
        .checked_mul(
            u64::try_from(input.len() + 1).map_err(|_| OperationError::OutputLimitExceeded)?,
        )
        .ok_or(OperationError::OutputLimitExceeded)?;
    if bound > context.budget().max_output_bytes {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut subsets: Vec<String> = Vec::with_capacity(
        usize::try_from(count).map_err(|_| OperationError::OutputLimitExceeded)?,
    );
    for mask in 0..count {
        if mask % 1024 == 0 {
            context.ensure_active()?;
        }
        // The reference builds the binary string most-significant digit first
        // and indexes items by position, so bit zero selects the last item.
        let mut chosen: Vec<&str> = Vec::new();
        for (position, item) in items.iter().enumerate() {
            let bit = items.len() - 1 - position;
            if (mask >> bit) & 1 == 1 {
                chosen.push(item);
            }
        }
        subsets.push(chosen.join(delimiter));
    }

    // Stable sort by length, matching the reference's comparator exactly.
    subsets.sort_by_key(alloc::string::String::len);

    let mut output = String::new();
    for subset in subsets {
        output.push_str(&subset);
        output.push('\n');
    }
    context.ensure_active()?;
    Ok(output)
}
