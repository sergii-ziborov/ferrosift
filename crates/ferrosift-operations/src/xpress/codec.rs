//! XPRESS (MS-XCA) decompression, both variants.

use alloc::{vec, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The reference's own ceiling on one call's output.
///
/// Part of the observable contract rather than a safety margin: a stream that
/// would produce more than this is an *error* in the reference, not a slow
/// success, so a port that merely allowed more would disagree about which
/// inputs are valid. Windows sizes XPRESS blocks at up to 32 MiB for WIM
/// chunks and 1 MiB for WOF chunks, which is where the number comes from.
const MAX_DECOMPRESSED: usize = 32 * 1024 * 1024;

/// The widest code the Huffman variant's canonical table holds.
const TABLE_BITS: u32 = 15;
const TABLE_SIZE: usize = 1 << TABLE_BITS;

const TRUNCATED_FLAG_GROUP: &str = "compression.xpress.truncated_flag_group";
const TRUNCATED_LITERAL: &str = "compression.xpress.truncated_literal";
const TRUNCATED_MATCH: &str = "compression.xpress.truncated_match";
const TRUNCATED_SHARED_NIBBLE: &str = "compression.xpress.truncated_shared_nibble";
const TRUNCATED_RAW_LENGTH: &str = "compression.xpress.truncated_raw_length";
const INVALID_MATCH_LENGTH: &str = "compression.xpress.invalid_match_length";
const OFFSET_OUT_OF_RANGE: &str = "compression.xpress.offset_out_of_range";
const RATIO_TOO_LARGE: &str = "compression.xpress.ratio_too_large";
const INVALID_SIZE: &str = "compression.xpress.invalid_decompressed_size";
const TRUNCATED_TABLE: &str = "compression.xpress.truncated_huffman_table";
const INVALID_CODE_LENGTHS: &str = "compression.xpress.invalid_code_lengths";
const TRUNCATED_BIT_STREAM: &str = "compression.xpress.truncated_bit_stream";
const CORRUPT_END_MARKER: &str = "compression.xpress.corrupt_end_marker";
const OUTPUT_EXCEEDS_SIZE: &str = "compression.xpress.output_exceeds_declared_size";

/// Decompresses an XPRESS plain-LZ77 stream (MS-XCA section 2.1).
///
/// The stream is self-terminating. Thirty-two-bit flag groups are tested from
/// bit 31 down; a clear bit is a literal and a set bit a match, and a set bit
/// with no input left is the end of the data rather than a truncation.
///
/// The shared-nibble form is the part a port gets wrong. A match whose low
/// three bits are seven takes an extra nibble, and that nibble is *half a
/// byte*: the first such match consumes the low nibble of a fresh byte and
/// leaves the high nibble for the next one, which may be many matches later.
/// The pending position is therefore an index into the input, not a saved
/// value, and it survives across matches.
///
/// # Errors
///
/// Refuses a truncated stream, a match pointing outside the window, and an
/// output past the reference's own ceiling or the caller's budget.
pub(super) fn decompress(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let budget_limit = output_limit(context);

    let mut output: Vec<u8> = Vec::new();
    // Where the half-used nibble byte sits, when one is outstanding.
    let mut pending: Option<usize> = None;
    let mut flags: u32 = 0;
    let mut flags_left: u32 = 0;
    let mut cursor = 0_usize;

    loop {
        if flags_left == 0 {
            // Once per thirty-two items is often enough to notice a
            // cancellation and rare enough not to be the cost of the loop.
            context.ensure_active()?;
            let Some(group) = input.get(cursor..cursor + 4) else {
                return Err(failed(TRUNCATED_FLAG_GROUP));
            };
            flags = u32::from_le_bytes([group[0], group[1], group[2], group[3]]);
            cursor += 4;
            flags_left = 32;
        }
        flags_left -= 1;

        if (flags >> flags_left) & 1 == 0 {
            let Some(byte) = input.get(cursor).copied() else {
                return Err(failed(TRUNCATED_LITERAL));
            };
            output.push(byte);
            cursor += 1;
            // Only the budget, not the reference's ceiling: the reference
            // checks that ceiling before a match and not after a literal, and
            // reaching it on literals alone would need an input past the
            // budget's own input limit anyway.
            if output.len() > budget_limit {
                return Err(OperationError::OutputLimitExceeded);
            }
            continue;
        }

        // The final flag group is padded with set bits, so a match flag with
        // nothing left to read is how the stream says it is over.
        if cursor >= input.len() {
            return Ok(output);
        }
        let Some(pair) = input.get(cursor..cursor + 2) else {
            return Err(failed(TRUNCATED_MATCH));
        };
        let encoded = u16::from_le_bytes([pair[0], pair[1]]);
        cursor += 2;
        let offset = usize::from(encoded >> 3) + 1;
        let mut length = usize::from(encoded & 7) + 3;

        if encoded & 7 == 7 {
            let nibble = if let Some(at) = pending.take() {
                u32::from(input[at] >> 4)
            } else {
                let Some(byte) = input.get(cursor).copied() else {
                    return Err(failed(TRUNCATED_SHARED_NIBBLE));
                };
                pending = Some(cursor);
                cursor += 1;
                u32::from(byte & 0x0f)
            };
            length = if nibble == 15 {
                match raw_length(input, &mut cursor)? {
                    // Only the escaped form carries a floor: a value the short
                    // form could have expressed is a stream that encoded the
                    // same length two ways, which the reference refuses.
                    RawLength::Escaped(value) => {
                        if value < 22 {
                            return Err(failed(INVALID_MATCH_LENGTH));
                        }
                        saturating_usize(value).saturating_add(3)
                    }
                    RawLength::Short(value) => usize::from(value) + 25,
                }
            } else {
                saturating_usize(nibble) + 10
            };
        }

        if offset > 8192 || offset > output.len() {
            return Err(failed(OFFSET_OUT_OF_RANGE));
        }
        ensure_room(output.len(), length, None, budget_limit)?;

        let start = output.len() - offset;
        // One byte at a time, because a run may point into the bytes it is
        // producing — which is how the format writes a repeat longer than the
        // distance behind it.
        for delta in 0..length {
            output.push(output[start + delta]);
        }
    }
}

/// Decompresses an XPRESS LZ77+Huffman stream (MS-XCA section 2.2).
///
/// Unlike the plain variant this stream does not say where it ends, so the
/// decompressed size is an argument: it comes from the WOF chunk table or the
/// WIM header in the places the format is actually used.
///
/// The Huffman table is 512 four-bit code lengths packed into the first 256
/// bytes, expanded into a flat 2^15-entry lookup in canonical (length, symbol)
/// order. The bit stream that follows is little-endian 16-bit words read
/// most-significant bit first — and raw match lengths are read as *bytes* from
/// the same cursor the bit reader is refilling from, interleaved with it.
///
/// # Errors
///
/// Refuses a size outside the reference's range, a code-length set that does
/// not fill the table exactly, a truncated stream, and an output that would
/// pass the declared size.
pub(super) fn decompress_huffman(
    input: &[u8],
    declared: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if declared <= 0 || declared > MAX_DECOMPRESSED as i128 {
        return Err(failed(INVALID_SIZE));
    }
    let declared = saturating_usize_from(declared);
    // The declared size is what the output grows to, so it is the allocation
    // the caller is really asking for and the one worth refusing up front.
    context.ensure_transient(declared as u64)?;
    if u64::try_from(declared).unwrap_or(u64::MAX) > context.budget().max_output_bytes {
        return Err(OperationError::OutputLimitExceeded);
    }
    if input.len() < 256 {
        return Err(failed(TRUNCATED_TABLE));
    }

    let mut lengths = [0_u8; 512];
    for (index, byte) in input[..256].iter().enumerate() {
        lengths[index * 2] = byte & 0x0f;
        lengths[index * 2 + 1] = byte >> 4;
    }

    context.ensure_transient((TABLE_SIZE * 2) as u64)?;
    let table = build_table(&lengths)?;

    let mut reader = BitReader {
        bits: 0,
        available: 0,
        cursor: 256,
    };
    while reader.available < 32 {
        reader.refill(input)?;
    }

    let mut output: Vec<u8> = Vec::new();
    let mut since_check = 0_u32;
    loop {
        since_check += 1;
        if since_check >= 4096 {
            context.ensure_active()?;
            since_check = 0;
        }
        while reader.available < TABLE_BITS {
            reader.refill(input)?;
        }
        let symbol = table[((reader.bits >> 17) & 0x7fff) as usize];
        reader.consume(u32::from(lengths[symbol as usize]));

        if symbol < 256 {
            // The branch is the proof: a symbol below 256 is a byte, which is
            // what makes this arm the literal one.
            output.push(u8::try_from(symbol).unwrap_or_default());
            if output.len() > declared {
                return Err(failed(OUTPUT_EXCEEDS_SIZE));
            }
            continue;
        }

        if symbol == 256 {
            // End of data — but only where the output is already the declared
            // length. Anywhere else the same symbol is an ordinary match of
            // three bytes at distance one, which is a quirk of the format
            // rather than a fallback this port invented.
            if output.len() == declared {
                break;
            }
            if output.is_empty() || declared - output.len() < 3 {
                return Err(failed(CORRUPT_END_MARKER));
            }
            let start = output.len() - 1;
            for delta in 0..3 {
                output.push(output[start + delta]);
            }
            continue;
        }

        let extra_bits = u32::from((symbol - 256) >> 4);
        let mut length = usize::from((symbol - 256) & 15);
        if length == 15 {
            length = match raw_length(input, &mut reader.cursor)? {
                // No floor here, unlike the plain variant: the two paths reach
                // the same escape through different bases and the reference
                // only refuses the redundant encoding in one of them.
                RawLength::Escaped(value) => saturating_usize(value).saturating_add(3),
                RawLength::Short(value) => usize::from(value) + 18,
            };
        } else {
            length += 3;
        }

        while reader.available < extra_bits {
            reader.refill(input)?;
        }
        let mut offset = 0_usize;
        if extra_bits > 0 {
            offset = ((reader.bits >> (32 - extra_bits)) & ((1_u32 << extra_bits) - 1)) as usize;
            reader.consume(extra_bits);
        }
        offset += 1_usize << extra_bits;

        if offset > output.len() {
            return Err(failed(OFFSET_OUT_OF_RANGE));
        }
        ensure_room(output.len(), length, Some(declared), usize::MAX)?;

        let start = output.len() - offset;
        for delta in 0..length {
            output.push(output[start + delta]);
        }
    }
    Ok(output)
}

/// The canonical decode table, one entry per 15-bit prefix.
///
/// A code set that does not fill the table exactly is refused. The reference
/// reaches the same verdict from the other end — it writes past its array and
/// then finds the fill count wrong — so an over-subscribed set and an
/// under-subscribed one give the same answer in both.
fn build_table(lengths: &[u8; 512]) -> Result<Vec<u16>, OperationError> {
    let mut table = vec![0_u16; TABLE_SIZE];
    let mut filled = 0_usize;
    for length in 1..=TABLE_BITS {
        for (symbol, code_length) in lengths.iter().enumerate() {
            if u32::from(*code_length) != length {
                continue;
            }
            let span = 1_usize << (TABLE_BITS - length);
            if filled + span > TABLE_SIZE {
                return Err(failed(INVALID_CODE_LENGTHS));
            }
            // The array is 512 long, so the index always fits.
            table[filled..filled + span].fill(u16::try_from(symbol).unwrap_or_default());
            filled += span;
        }
    }
    if filled != TABLE_SIZE {
        return Err(failed(INVALID_CODE_LENGTHS));
    }
    Ok(table)
}

/// A thirty-two-bit register whose valid bits are the topmost `available`.
struct BitReader {
    bits: u32,
    available: u32,
    cursor: usize,
}

impl BitReader {
    /// Adds one little-endian 16-bit word below the bits already held.
    fn refill(&mut self, input: &[u8]) -> Result<(), OperationError> {
        let Some(pair) = input.get(self.cursor..self.cursor + 2) else {
            return Err(failed(TRUNCATED_BIT_STREAM));
        };
        let word = u32::from(u16::from_le_bytes([pair[0], pair[1]]));
        // Callers only refill below sixteen held bits, so the shift is in
        // range and the word lands where the register is still empty.
        self.bits |= word << (16 - self.available);
        self.cursor += 2;
        self.available += 16;
        Ok(())
    }

    fn consume(&mut self, count: u32) {
        self.bits <<= count;
        self.available -= count;
    }
}

/// The two shapes an escaped match length arrives in.
enum RawLength {
    /// A single byte below 255, whose meaning depends on the variant.
    Short(u8),
    /// The escaped form: an LE16, or an LE32 when that LE16 is zero.
    Escaped(u32),
}

/// Reads an escaped match length from the byte stream.
///
/// Shared because both variants encode it identically — a byte, an LE16 when
/// that byte is 255, an LE32 when the LE16 is zero — and differ only in what
/// they add to it afterwards.
fn raw_length(input: &[u8], cursor: &mut usize) -> Result<RawLength, OperationError> {
    let Some(first) = input.get(*cursor).copied() else {
        return Err(failed(TRUNCATED_RAW_LENGTH));
    };
    *cursor += 1;
    if first != 255 {
        return Ok(RawLength::Short(first));
    }

    let Some(pair) = input.get(*cursor..*cursor + 2) else {
        return Err(failed(TRUNCATED_RAW_LENGTH));
    };
    let mut value = u32::from(u16::from_le_bytes([pair[0], pair[1]]));
    *cursor += 2;
    if value == 0 {
        let Some(quad) = input.get(*cursor..*cursor + 4) else {
            return Err(failed(TRUNCATED_RAW_LENGTH));
        };
        value = u32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]);
        *cursor += 4;
    }
    Ok(RawLength::Escaped(value))
}

/// Refuses an output about to pass the reference's ceiling, the declared size,
/// or the caller's budget — in that order, so each refusal names its own cause.
fn ensure_room(
    produced: usize,
    adding: usize,
    declared: Option<usize>,
    budget_limit: usize,
) -> Result<(), OperationError> {
    let total = produced.saturating_add(adding);
    if total > MAX_DECOMPRESSED {
        return Err(failed(RATIO_TOO_LARGE));
    }
    if declared.is_some_and(|size| total > size) {
        return Err(failed(OUTPUT_EXCEEDS_SIZE));
    }
    if total > budget_limit {
        return Err(OperationError::OutputLimitExceeded);
    }
    Ok(())
}

/// The caller's output ceiling, as a length this decoder can compare against.
///
/// Saturating rather than failing: on a target whose `usize` is narrower than
/// the budget's `u64`, a ceiling the index type cannot express is one the
/// allocation would hit first, and the check after decompression is then the
/// one that holds.
fn output_limit(context: &OperationContext<'_>) -> usize {
    usize::try_from(context.budget().max_output_bytes).unwrap_or(usize::MAX)
}

fn saturating_usize(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn saturating_usize_from(value: i128) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
