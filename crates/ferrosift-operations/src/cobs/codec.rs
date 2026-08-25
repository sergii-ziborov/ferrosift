//! Consistent Overhead Byte Stuffing, as the reference implements it.

use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;

/// The span a maximal block covers, counting its own length byte.
const MAX_BLOCK: usize = 0xFF;

/// The most literal bytes one block can carry.
const MAX_PAYLOAD: usize = MAX_BLOCK - 1;

/// COBS-encodes a byte string.
///
/// The reference prepends a zero and then repeats: find the next zero, emit its
/// distance and the bytes before it, drop what was consumed. This mirrors that
/// shape rather than the more familiar running-pointer formulation, because the
/// two disagree about where a maximal run splits and the reference's answer is
/// the one that has to come out.
#[must_use]
pub fn encode(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(input.len() + input.len() / MAX_PAYLOAD + 2);
    // The leading zero is the sentinel each length is measured from. It is
    // never emitted; it only marks where the current block starts.
    let mut data: Vec<u8> = Vec::with_capacity(input.len() + 1);
    data.push(0);
    data.extend_from_slice(input);

    while !data.is_empty() {
        let end = data
            .iter()
            .skip(1)
            .position(|byte| *byte == 0)
            .map(|at| at + 1);

        if end.is_none_or(|at| at > MAX_PAYLOAD) && data.len() > MAX_PAYLOAD {
            // The run is longer than one length byte can describe. Emit a
            // maximal block and re-prepend the sentinel, because the run has
            // not ended — only the block has.
            output.push(0xFF);
            output.extend_from_slice(&data[1..MAX_BLOCK]);
            data.drain(..MAX_BLOCK);
            if !data.is_empty() {
                data.insert(0, 0);
            }
            continue;
        }

        // Both remaining arms fit in a byte: the branch above already took
        // every case where the block would exceed 254.
        let (length, consumed) = match end {
            None => (data.len(), data.len()),
            Some(at) => (at, at),
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the branch above leaves at most 254 here"
        )]
        output.push(length as u8);
        output.extend_from_slice(&data[1..length]);
        data.drain(..consumed);
    }

    output
}

/// COBS-decodes a byte string.
///
/// A zero byte anywhere in the payload is the reference's only rejection.
/// Everything else is decoded as far as it goes: a truncated final block yields
/// the bytes it does contain rather than an error, which is why `[0x05]` alone
/// decodes to nothing instead of complaining about the four bytes it promised.
///
/// # Errors
///
/// Returns an error when the payload contains a zero byte.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, OperationError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if input.contains(&0) {
        return Err(failed("encoding.cobs.zero_byte"));
    }

    let mut output = Vec::with_capacity(input.len());
    let mut data = input;

    while !data.is_empty() {
        if data[0] == 0xFF {
            // A leading maximal block stands for literal bytes with no zero
            // after them, so nothing separates it from what follows.
            let span = MAX_BLOCK.min(data.len());
            output.extend_from_slice(&data[1..span]);
            data = &data[span..];
            continue;
        }

        // Rejecting zero above guarantees every length here is at least one,
        // so each step consumes something and the loops terminate.
        let span = usize::from(data[0]).min(data.len());
        output.extend_from_slice(&data[1..span]);
        data = &data[span..];

        // Each further block in this chain begins at a byte that was a zero,
        // so it emits that zero first. A maximal block ends the chain: the
        // bytes after it had no zero between them.
        while let Some(block) = data.first().copied() {
            let span = usize::from(block).min(data.len());
            output.push(0);
            output.extend_from_slice(&data[1..span]);
            data = &data[span..];
            if block == 0xFF {
                break;
            }
        }
    }

    Ok(output)
}
