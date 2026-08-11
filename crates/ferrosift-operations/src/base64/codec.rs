use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use super::alphabet::{Alphabet, failed};

pub(super) fn encode(
    input: &[u8],
    alphabet: &Alphabet,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let capacity = encoded_len(input.len(), alphabet.padding().is_some())
        .ok_or(OperationError::OutputLimitExceeded)?;
    ensure_output_fits(capacity, context)?;
    let mut output = String::with_capacity(capacity);
    for (index, chunk) in input.chunks(3).enumerate() {
        if index % 1366 == 0 {
            context.ensure_active()?;
        }
        let first = chunk[0];
        output.push(alphabet.symbol(usize::from(first >> 2)));
        let second_index = (first & 0x03) << 4 | chunk.get(1).copied().unwrap_or(0) >> 4;
        output.push(alphabet.symbol(usize::from(second_index)));
        if let Some(second) = chunk.get(1).copied() {
            let third_index = (second & 0x0f) << 2 | chunk.get(2).copied().unwrap_or(0) >> 6;
            output.push(alphabet.symbol(usize::from(third_index)));
        } else if let Some(padding) = alphabet.padding() {
            output.push(padding);
        }
        if let Some(third) = chunk.get(2).copied() {
            output.push(alphabet.symbol(usize::from(third & 0x3f)));
        } else if let Some(padding) = alphabet.padding() {
            output.push(padding);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    alphabet: &Alphabet,
    remove_non_alphabet: bool,
    strict: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut symbols = Vec::with_capacity(input.len());
    for value in input.chars() {
        if alphabet.contains(value) {
            symbols.push(value);
        } else if !remove_non_alphabet {
            return Err(failed("encoding.base64.invalid_character"));
        }
    }
    validate_shape(&symbols, alphabet)?;
    if strict {
        validate_canonical_bits(&symbols, alphabet)?;
    }
    let capacity = decoded_len(&symbols, alphabet)?;
    ensure_output_fits(capacity, context)?;
    let mut output = Vec::with_capacity(capacity);
    for (index, chunk) in symbols.chunks(4).enumerate() {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        decode_chunk(chunk, alphabet, &mut output)?;
    }
    context.ensure_active()?;
    Ok(output)
}

fn encoded_len(input_len: usize, padded: bool) -> Option<usize> {
    if padded {
        input_len.checked_add(2)?.checked_div(3)?.checked_mul(4)
    } else {
        let complete = input_len.checked_div(3)?.checked_mul(4)?;
        complete.checked_add(match input_len % 3 {
            0 => 0,
            1 => 2,
            _ => 3,
        })
    }
}

fn decoded_len(symbols: &[char], alphabet: &Alphabet) -> Result<usize, OperationError> {
    if symbols.is_empty() {
        return Ok(0);
    }
    let padding = alphabet.padding();
    let padding_count = symbols
        .iter()
        .rev()
        .take_while(|value| Some(**value) == padding)
        .count();
    let complete = symbols.len() / 4;
    let remainder = symbols.len() % 4;
    complete
        .checked_mul(3)
        .and_then(|value| {
            value.checked_add(match remainder {
                2 => 1,
                3 => 2,
                _ => 0,
            })
        })
        .and_then(|value| value.checked_sub(padding_count))
        .ok_or(OperationError::OutputLimitExceeded)
}

fn validate_shape(symbols: &[char], alphabet: &Alphabet) -> Result<(), OperationError> {
    if symbols.len() % 4 == 1 {
        return Err(failed("encoding.base64.invalid_length"));
    }
    let Some(padding) = alphabet.padding() else {
        return Ok(());
    };
    let Some(first_padding) = symbols.iter().position(|value| *value == padding) else {
        return Ok(());
    };
    let padding_count = symbols.len() - first_padding;
    if padding_count > 2
        || symbols[first_padding..]
            .iter()
            .any(|value| *value != padding)
        || !symbols.len().is_multiple_of(4)
    {
        return Err(failed("encoding.base64.invalid_padding"));
    }
    Ok(())
}

fn validate_canonical_bits(symbols: &[char], alphabet: &Alphabet) -> Result<(), OperationError> {
    let unpadded_len = symbols
        .iter()
        .position(|value| Some(*value) == alphabet.padding())
        .unwrap_or(symbols.len());
    match unpadded_len % 4 {
        2 => {
            let value = value(symbols[unpadded_len - 1], alphabet)?;
            if value & 0x0f != 0 {
                return Err(failed("encoding.base64.non_canonical"));
            }
        }
        3 => {
            let value = value(symbols[unpadded_len - 1], alphabet)?;
            if value & 0x03 != 0 {
                return Err(failed("encoding.base64.non_canonical"));
            }
        }
        _ => {}
    }
    Ok(())
}

fn decode_chunk(
    chunk: &[char],
    alphabet: &Alphabet,
    output: &mut Vec<u8>,
) -> Result<(), OperationError> {
    if chunk.len() < 2 {
        return Err(failed("encoding.base64.invalid_length"));
    }
    let first = value(chunk[0], alphabet)?;
    let second = value(chunk[1], alphabet)?;
    output.push((first << 2) | (second >> 4));

    if let Some(third) = chunk.get(2).copied() {
        if Some(third) == alphabet.padding() {
            return Ok(());
        }
        let third = value(third, alphabet)?;
        output.push((second << 4) | (third >> 2));
        if let Some(fourth) = chunk.get(3).copied()
            && Some(fourth) != alphabet.padding()
        {
            output.push((third << 6) | value(fourth, alphabet)?);
        }
    }
    Ok(())
}

fn value(symbol: char, alphabet: &Alphabet) -> Result<u8, OperationError> {
    alphabet
        .value(symbol)
        .ok_or_else(|| failed("encoding.base64.invalid_character"))
}

fn ensure_output_fits(
    output_size: usize,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    let size = u64::try_from(output_size).map_err(|_| OperationError::OutputLimitExceeded)?;
    if size > context.budget().max_output_bytes {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
