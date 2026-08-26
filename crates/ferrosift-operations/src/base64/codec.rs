use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use super::alphabet::{Alphabet, failed};

/// Groups of three encoded between two checks of the cancellation flag.
const TRIPLES_PER_BLOCK: usize = 1366;

/// Groups of three staged before a write reaches the output.
///
/// Deliberately much smaller than the cancellation block, and the two are
/// separate for a reason worth stating. Sizing the staging buffer to the
/// block made it five kilobytes, which a sixteen-byte input then zeroed in
/// full before encoding five groups -- and measurably lost, at the small
/// sizes, everything the change won at the large ones. Sixty-four groups is
/// a quarter-kilobyte of stack: still one capacity check per two hundred and
/// fifty-six output bytes rather than per four, and nothing a short input
/// notices.
const STAGING_TRIPLES: usize = 64;

pub(super) fn encode(
    input: &[u8],
    alphabet: &Alphabet,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let capacity = encoded_len(input.len(), alphabet.padding().is_some())
        .ok_or(OperationError::OutputLimitExceeded)?;
    ensure_output_fits(capacity, context)?;
    let mut output = Vec::with_capacity(capacity);
    // Whole triples first. Splitting the tail off means this loop never
    // bounds-checks a partial chunk and never consults the padding, which is
    // what the previous single loop paid for on every group of three.
    let whole = input.len() - input.len() % 3;
    let (body, tail) = input.split_at(whole);

    // Symbols land in a fixed stack buffer and reach the output in batches.
    // Writing them straight into the `Vec` meant a capacity check per group of
    // three -- a third of a million of them on a megabyte -- for a buffer
    // whose size was already known before the loop began. Filling an array of
    // known length instead lets the four stores be seen as four stores.
    //
    // A zeroed output buffer of the full size would remove the checks too, and
    // pay a megabyte of memset to do it. This pays a quarter of a kilobyte.
    let mut staging = [0_u8; STAGING_TRIPLES * 4];
    for block in body.chunks(3 * TRIPLES_PER_BLOCK) {
        context.ensure_active()?;
        for group in block.chunks(3 * STAGING_TRIPLES) {
            for (triple, quad) in group.chunks_exact(3).zip(staging.chunks_exact_mut(4)) {
                let packed =
                    u32::from(triple[0]) << 16 | u32::from(triple[1]) << 8 | u32::from(triple[2]);
                quad[0] = alphabet.symbol_byte((packed >> 18) as usize & 0x3f);
                quad[1] = alphabet.symbol_byte((packed >> 12) as usize & 0x3f);
                quad[2] = alphabet.symbol_byte((packed >> 6) as usize & 0x3f);
                quad[3] = alphabet.symbol_byte(packed as usize & 0x3f);
            }
            output.extend_from_slice(&staging[..group.len() / 3 * 4]);
        }
    }

    if !tail.is_empty() {
        let first = tail[0];
        output.push(alphabet.symbol_byte(usize::from(first >> 2)));
        let second = tail.get(1).copied();
        output.push(
            alphabet.symbol_byte(usize::from((first & 0x03) << 4 | second.unwrap_or(0) >> 4)),
        );
        match second {
            Some(second) => output.push(alphabet.symbol_byte(usize::from((second & 0x0f) << 2))),
            None => {
                if let Some(padding) = alphabet.padding_byte() {
                    output.push(padding);
                }
            }
        }
        if let Some(padding) = alphabet.padding_byte() {
            output.push(padding);
        }
    }

    context.ensure_active()?;
    // Every byte came from the alphabet, which parsing validated as ASCII, so
    // this scan cannot fail. It is here rather than an unchecked conversion
    // because the crate forbids unsafe.
    String::from_utf8(output).map_err(|_| failed("encoding.base64.invalid_alphabet"))
}

pub(super) fn decode(
    input: &str,
    alphabet: &Alphabet,
    remove_non_alphabet: bool,
    strict: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    // Bytes, not `char`. Every alphabet symbol is ASCII, so a byte at or above
    // 0x80 — which is every byte of a multi-byte sequence — can never be in
    // the alphabet, and is filtered or rejected exactly as the character it
    // belongs to would have been. Collecting `char` cost four bytes per symbol
    // and a UTF-8 decode of the whole input before any decoding began.
    // Scanned before it is copied. Well-formed base64 — which is nearly all of
    // it — has nothing to filter, and the previous version still allocated a
    // second buffer the size of the input and copied every byte into it to
    // discover that. Finding the first byte that needs removing costs one pass
    // with no allocation, and when there is none the input is used where it
    // lies.
    let first_foreign = input
        .bytes()
        .position(|value| !alphabet.contains_byte(value));
    let owned;
    let symbols: &[u8] = match first_foreign {
        None => input.as_bytes(),
        Some(at) => {
            if !remove_non_alphabet {
                return Err(failed("encoding.base64.invalid_character"));
            }
            // Everything before the first foreign byte is already known good,
            // so the copy starts there rather than at the beginning.
            let mut kept = Vec::with_capacity(input.len() - 1);
            kept.extend_from_slice(&input.as_bytes()[..at]);
            for value in input.as_bytes()[at + 1..].iter().copied() {
                if alphabet.contains_byte(value) {
                    kept.push(value);
                }
            }
            owned = kept;
            &owned
        }
    };
    validate_shape(symbols, alphabet)?;
    if strict {
        validate_canonical_bits(symbols, alphabet)?;
    }
    let capacity = decoded_len(symbols, alphabet)?;
    ensure_output_fits(capacity, context)?;
    let mut output = Vec::with_capacity(capacity);

    // `validate_shape` has already established that padding, if present at
    // all, occupies only the last one or two positions. Stripping it here
    // means the loop below never compares against it — the previous version
    // asked twice per group of four, for the sake of the final group.
    let data = match alphabet.padding_byte() {
        Some(padding) => {
            let end = symbols
                .iter()
                .rposition(|value| *value != padding)
                .map_or(0, |index| index + 1);
            &symbols[..end]
        }
        None => symbols,
    };

    let whole = data.len() - data.len() % 4;
    let (body, tail) = data.split_at(whole);
    for block in body.chunks(4 * 1024) {
        context.ensure_active()?;
        for quad in block.chunks_exact(4) {
            let packed = u32::from(value(quad[0], alphabet)?) << 18
                | u32::from(value(quad[1], alphabet)?) << 12
                | u32::from(value(quad[2], alphabet)?) << 6
                | u32::from(value(quad[3], alphabet)?);
            // Three bytes in one capacity check rather than three. Each shift
            // leaves the wanted byte in the low bits, so the masks are what
            // make the narrowing exact rather than a truncation.
            output.extend_from_slice(&[
                ((packed >> 16) & 0xff) as u8,
                ((packed >> 8) & 0xff) as u8,
                (packed & 0xff) as u8,
            ]);
        }
    }

    // A tail of two symbols carries one byte, three carry two. One symbol is
    // impossible: `validate_shape` rejects that length.
    if !tail.is_empty() {
        let first = value(tail[0], alphabet)?;
        let second = value(tail[1], alphabet)?;
        output.push((first << 2) | (second >> 4));
        if let Some(third) = tail.get(2).copied() {
            output.push((second << 4) | (value(third, alphabet)? >> 2));
        }
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

fn decoded_len(symbols: &[u8], alphabet: &Alphabet) -> Result<usize, OperationError> {
    if symbols.is_empty() {
        return Ok(0);
    }
    let padding = alphabet.padding_byte();
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

fn validate_shape(symbols: &[u8], alphabet: &Alphabet) -> Result<(), OperationError> {
    if symbols.len() % 4 == 1 {
        return Err(failed("encoding.base64.invalid_length"));
    }
    let Some(padding) = alphabet.padding_byte() else {
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

fn validate_canonical_bits(symbols: &[u8], alphabet: &Alphabet) -> Result<(), OperationError> {
    let unpadded_len = symbols
        .iter()
        .position(|value| Some(*value) == alphabet.padding_byte())
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

fn value(symbol: u8, alphabet: &Alphabet) -> Result<u8, OperationError> {
    alphabet
        .value_byte(symbol)
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
