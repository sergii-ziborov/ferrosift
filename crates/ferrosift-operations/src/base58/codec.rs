use alloc::{collections::BTreeSet, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_ALPHABET: &str = "encoding.base58.invalid_alphabet";
const INVALID_CHARACTER: &str = "encoding.base58.invalid_character";

pub(super) fn parse_alphabet(expression: &str) -> Result<Vec<char>, OperationError> {
    let alphabet = crate::alphabet::expand(expression, INVALID_ALPHABET)?;
    let mut unique = BTreeSet::new();
    if alphabet.len() != 58
        || !alphabet.iter().all(char::is_ascii)
        || !alphabet.iter().all(|value| unique.insert(*value))
    {
        return Err(failed(INVALID_ALPHABET));
    }
    Ok(alphabet)
}

/// Encodes with the carry-propagation big-number walk the reference uses;
/// the work grows quadratically with the input, so cancellation is checked
/// on every outer byte batch.
pub(super) fn encode(
    input: &[u8],
    alphabet: &[char],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let bound = input
        .len()
        .checked_mul(2)
        .ok_or(OperationError::OutputLimitExceeded)?;
    if u64::try_from(bound).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let zero_prefix = input.iter().take_while(|byte| **byte == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    for (index, byte) in input.iter().enumerate() {
        if index % 256 == 0 {
            context.ensure_active()?;
        }
        let mut carry = u32::from(*byte);
        for digit in &mut digits {
            carry += u32::from(*digit) << 8;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    let mut output = String::with_capacity(zero_prefix + digits.len());
    for _ in 0..zero_prefix {
        output.push(alphabet[0]);
    }
    for digit in digits.iter().rev() {
        output.push(alphabet[usize::from(*digit)]);
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    alphabet: &[char],
    remove_non_alphabet: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    // The reference counts leading zero symbols on the raw input before any
    // non-alphabet characters are removed, so noise ahead of the zeros
    // suppresses them; that quirk is observable and preserved.
    let zero_prefix = input
        .chars()
        .take_while(|symbol| *symbol == alphabet[0])
        .count();

    let mut bytes: Vec<u8> = Vec::new();
    for (position, symbol) in input.chars().enumerate() {
        if position % 256 == 0 {
            context.ensure_active()?;
        }
        let Some(index) = alphabet.iter().position(|value| *value == symbol) else {
            if remove_non_alphabet {
                continue;
            }
            return Err(failed(INVALID_CHARACTER));
        };
        let mut carry = u32::try_from(index).map_err(|_| failed(INVALID_CHARACTER))?;
        for byte in &mut bytes {
            carry += u32::from(*byte) * 58;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push((carry & 0xff) as u8);
            carry >>= 8;
        }
        if bytes.len() as u64 > context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
    }

    if (bytes.len() + zero_prefix) as u64 > context.budget().max_output_bytes {
        return Err(OperationError::OutputLimitExceeded);
    }
    bytes.resize(bytes.len() + zero_prefix, 0);
    bytes.reverse();
    context.ensure_active()?;
    Ok(bytes)
}
