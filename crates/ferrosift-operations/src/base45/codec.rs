use alloc::{collections::BTreeSet, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_ALPHABET: &str = "encoding.base45.invalid_alphabet";
const INVALID_CHARACTER: &str = "encoding.base45.invalid_character";
const TRIPLET_OVERFLOW: &str = "encoding.base45.triplet_overflow";

pub(super) fn encode(
    input: &[u8],
    expression: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let alphabet = crate::alphabet::expand(expression, INVALID_ALPHABET)?;
    let mut unique = BTreeSet::new();
    if alphabet.len() != 45
        || !alphabet.iter().all(char::is_ascii)
        || !alphabet.iter().all(|value| unique.insert(*value))
    {
        return Err(failed(INVALID_ALPHABET));
    }

    let capacity = input
        .len()
        .div_ceil(2)
        .checked_mul(3)
        .ok_or(OperationError::OutputLimitExceeded)?;
    if u64::try_from(capacity).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut output = String::with_capacity(capacity);
    for (index, pair) in input.chunks(2).enumerate() {
        if index % 2048 == 0 {
            context.ensure_active()?;
        }
        let mut value = pair.iter().fold(0_u32, |accumulator, byte| {
            accumulator * 256 + u32::from(*byte)
        });
        let mut emitted = 0_usize;
        loop {
            output.push(alphabet[(value % 45) as usize]);
            emitted += 1;
            value /= 45;
            if value == 0 {
                break;
            }
        }
        // The reference pads with a literal "0" character, not with the
        // first alphabet symbol; custom alphabets keep that quirk.
        if emitted < 2 {
            output.push('0');
            emitted += 1;
        }
        if pair.len() > 1 && emitted < 3 {
            output.push('0');
        }
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    expression: &str,
    remove_non_alphabet: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    // The reference performs no length or uniqueness validation when
    // decoding; any expanded alphabet is indexed as-is.
    let alphabet = crate::alphabet::expand(expression, INVALID_ALPHABET)?;
    let index_of = |symbol: char| alphabet.iter().position(|value| *value == symbol);

    let symbols: Vec<char> = if remove_non_alphabet {
        input
            .chars()
            .filter(|value| index_of(*value).is_some())
            .collect()
    } else {
        input.chars().collect()
    };

    let mut output = Vec::new();
    for (index, triple) in symbols.chunks(3).enumerate() {
        if index % 2048 == 0 {
            context.ensure_active()?;
        }
        let mut value = 0_u64;
        for symbol in triple.iter().rev() {
            let digit = index_of(*symbol).ok_or_else(|| failed(INVALID_CHARACTER))?;
            value = value.saturating_mul(45).saturating_add(digit as u64);
        }
        let value = u16::try_from(value).map_err(|_| failed(TRIPLET_OVERFLOW))?;
        if output.len() as u64 + 2 > context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
        let bytes = value.to_be_bytes();
        if triple.len() > 2 {
            output.push(bytes[0]);
        }
        // Short trailing groups keep only the low byte, mirroring the
        // reference's unconditional `& 0xff` mask.
        output.push(bytes[1]);
    }
    context.ensure_active()?;
    Ok(output)
}
