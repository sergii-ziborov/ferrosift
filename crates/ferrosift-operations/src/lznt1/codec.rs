//! LZNT1, the compression behind `RtlDecompressBuffer`.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// Set in a block header when the block's contents are compressed.
const COMPRESSED: u16 = 1 << 15;

/// The block header's low twelve bits hold the size, one less than the truth.
const SIZE: u16 = (1 << 12) - 1;

/// How many bits of a back-reference are given to the distance.
///
/// The split between distance and length moves as the window fills: early in a
/// block there is little to point back at, so most of the field is length.
/// This is what makes the format's back-references position-dependent, and
/// what a decoder gets wrong by assuming a fixed split.
fn displacement(offset: usize) -> u32 {
    let mut offset = offset;
    let mut result = 0;
    while offset >= 0x10 {
        offset >>= 1;
        result += 1;
    }
    result
}

/// Decompresses an LZNT1 stream.
///
/// A zero size ends the stream rather than failing: that is how a compressor
/// marks the end when the buffer it was given had room to spare.
pub(super) fn decompress(
    compressed: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let mut output: Vec<u8> = Vec::new();
    let mut cursor = 0;

    while cursor + 2 <= compressed.len() {
        context.ensure_active()?;
        let block_start = output.len();
        let header = u16::from_le_bytes([compressed[cursor], compressed[cursor + 1]]);
        cursor += 2;

        let size = usize::from(header & SIZE);
        let block_end = cursor + size + 1;

        if size == 0 {
            break;
        }
        if compressed.len() < cursor + size {
            return Err(failed("compression.lznt1.truncated_block"));
        }

        if header & COMPRESSED == 0 {
            let end = (cursor + size + 1).min(compressed.len());
            output.extend_from_slice(&compressed[cursor..end]);
            cursor += size + 1;
            continue;
        }

        while cursor < block_end {
            let Some(flags) = compressed.get(cursor).copied() else {
                break;
            };
            cursor += 1;
            let mut flags = flags;

            // Eight items per flag byte, low bit first: clear means one
            // literal byte, set means a two-byte back-reference.
            for _ in 0..8 {
                if cursor >= block_end {
                    break;
                }
                if flags & 1 == 0 {
                    let Some(byte) = compressed.get(cursor).copied() else {
                        break;
                    };
                    output.push(byte);
                    cursor += 1;
                } else {
                    let high = compressed.get(cursor + 1).copied().unwrap_or(0);
                    let low = compressed.get(cursor).copied().unwrap_or(0);
                    let pointer = u16::from_le_bytes([low, high]);
                    cursor += 2;

                    // A back-reference as the first item of a block asks for
                    // the displacement of minus one. The reference computes
                    // that in floating point, where the loop's `>= 0x10` is
                    // false at once and the answer is zero; saturating to zero
                    // reaches the same place without leaving the index type.
                    let consumed = output.len().saturating_sub(block_start).saturating_sub(1);
                    let split = displacement(consumed).min(12);
                    let distance = usize::from(pointer >> (12 - split)) + 1;
                    let length = usize::from(pointer & (0xfff >> split)) + 2;
                    let start = output.len().wrapping_sub(distance);

                    // Copied one byte at a time on purpose: a run may point
                    // into the bytes it is producing, which is how the format
                    // encodes a repeat longer than the distance behind it.
                    for delta in 0..=length {
                        let shift = start.wrapping_add(delta);
                        if shift >= output.len() {
                            return Err(failed("compression.lznt1.invalid_shift"));
                        }
                        output.push(output[shift]);
                    }
                }
                flags >>= 1;
            }
        }
    }

    Ok(output)
}
