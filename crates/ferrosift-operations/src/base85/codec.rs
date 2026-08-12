use alloc::{collections::BTreeSet, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_ALPHABET: &str = "encoding.base85.invalid_alphabet";
const INVALID_CHARACTER: &str = "encoding.base85.invalid_character";
const ZERO_CHARACTER_CONFLICT: &str = "encoding.base85.zero_character_conflict";

pub(super) fn parse_alphabet(expression: &str) -> Result<Vec<char>, OperationError> {
    let alphabet = crate::alphabet::expand(expression, INVALID_ALPHABET)?;
    let mut unique = BTreeSet::new();
    if alphabet.len() != 85
        || !alphabet.iter().all(char::is_ascii)
        || !alphabet.iter().all(|value| unique.insert(*value))
    {
        return Err(failed(INVALID_ALPHABET));
    }
    Ok(alphabet)
}

fn is_standard(alphabet: &[char]) -> bool {
    alphabet.iter().copied().eq('!'..='u')
}

pub(super) fn encode(
    input: &[u8],
    alphabet: &[char],
    include_delimiter: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let capacity = input
        .len()
        .div_ceil(4)
        .checked_mul(5)
        .and_then(|size| size.checked_add(4))
        .ok_or(OperationError::OutputLimitExceeded)?;
    if u64::try_from(capacity).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let standard = is_standard(alphabet);
    let mut output = String::with_capacity(capacity);
    if include_delimiter {
        output.push_str("<~");
    }
    for (index, group) in input.chunks(4).enumerate() {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        let block = group
            .iter()
            .enumerate()
            .fold(0_u32, |accumulator, (offset, byte)| {
                accumulator | (u32::from(*byte) << (24 - 8 * offset))
            });
        // The reference emits `z` for any all-zero block in the standard
        // alphabet, including a partial trailing block.
        if standard && block == 0 {
            output.push('z');
            continue;
        }
        let mut digits = [0_u32; 5];
        let mut remaining = block;
        for slot in digits.iter_mut().rev() {
            *slot = remaining % 85;
            remaining /= 85;
        }
        for digit in digits.iter().take(group.len() + 1) {
            output.push(alphabet[*digit as usize]);
        }
    }
    if include_delimiter {
        output.push_str("~>");
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    alphabet: &[char],
    remove_non_alphabet: bool,
    zero_group_character: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let zero_char = zero_group_character.chars().next();
    if zero_char.is_some_and(|value| alphabet.contains(&value)) {
        return Err(failed(ZERO_CHARACTER_CONFLICT));
    }

    let mut text = strip_delimiters(input);
    let filtered: String;
    if remove_non_alphabet {
        filtered = text
            .chars()
            .filter(|value| *value == '~' || Some(*value) == zero_char || alphabet.contains(value))
            .collect();
        text = strip_delimiters(&filtered);
    }

    let symbols: Vec<char> = text.chars().collect();
    let mut output = Vec::new();
    let mut index = 0;
    let mut iterations = 0_usize;
    while index < symbols.len() {
        if iterations.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        iterations += 1;
        if output.len() as u64 + 4 > context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
        if Some(symbols[index]) == zero_char {
            output.extend_from_slice(&[0, 0, 0, 0]);
            index += 1;
            continue;
        }
        let group = &symbols[index..symbols.len().min(index + 5)];
        let mut digits = [84_i64; 5];
        for (offset, symbol) in group.iter().enumerate() {
            digits[offset] = if Some(*symbol) == zero_char {
                // A zero-group character inside a block keeps the raw `-1`
                // index the reference feeds into the block arithmetic.
                -1
            } else {
                alphabet
                    .iter()
                    .position(|value| value == symbol)
                    .and_then(|position| i64::try_from(position).ok())
                    .ok_or_else(|| failed(INVALID_CHARACTER))?
            };
        }
        // A lone trailing symbol contributes nothing: the reference's block
        // arithmetic degenerates to NaN and every byte is spliced away.
        if group.len() > 1 {
            let block = digits[0] * 52_200_625
                + digits[1] * 614_125
                + digits[2] * 7_225
                + digits[3] * 85
                + digits[4];
            // Blocks above 2^32 (and negative blocks from an embedded
            // zero-group symbol) wrap exactly like the reference's 32-bit
            // shift coercion.
            let bytes = block.rem_euclid(1_i64 << 32).to_be_bytes();
            output.extend_from_slice(&bytes[4..4 + group.len() - 1]);
        }
        index += 5;
    }
    context.ensure_active()?;
    Ok(output)
}

/// Removes one `<~ ... ~>` wrapper when the whole input is wrapped, exactly
/// like the reference's anchored `/^<~(.+?)~>$/` (whose `.` cannot cross
/// line terminators and needs at least one inner character).
fn strip_delimiters(input: &str) -> &str {
    let stripped = input
        .strip_prefix("<~")
        .and_then(|value| value.strip_suffix("~>"));
    match stripped {
        Some(inner)
            if !inner.is_empty()
                && !inner
                    .chars()
                    .any(|value| matches!(value, '\n' | '\r' | '\u{2028}' | '\u{2029}')) =>
        {
            inner
        }
        _ => input,
    }
}
