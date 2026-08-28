use crate::ast::Endian;
use crate::error::{PatternError, Position};

use super::source::{ByteSource, MAX_SCALAR_BYTES};

pub(super) const OUT_OF_BOUNDS: &str = "pattern.eval.out_of_bounds";
pub(super) const SOURCE_FAILED: &str = "pattern.eval.source_failed";

/// Reads fixed-width scalars out of whatever the pattern is being evaluated
/// against.
///
/// Every read is bounds-checked against the source's own length before the
/// source is asked for it, so a pattern can never observe bytes that are not
/// there — and an implementation of [`ByteSource`] never has to defend itself
/// against a range the evaluator should have refused.
///
/// The bytes land in a stack buffer rather than a borrowed slice. That is what
/// makes a non-contiguous source possible at all: a block cache or a device
/// window has no `&[u8]` spanning an arbitrary offset to hand back, and asking
/// for one is what tied this to buffers that fit in memory.
pub(super) struct Reader<'a, S: ?Sized> {
    source: &'a S,
}

impl<'a, S: ByteSource + ?Sized> Reader<'a, S> {
    pub(super) const fn new(source: &'a S) -> Self {
        Self { source }
    }

    pub(super) fn len(&self) -> u64 {
        self.source.len()
    }

    /// Reads `size` bytes at `offset` as an unsigned integer.
    pub(super) fn unsigned(
        &self,
        offset: u64,
        size: u32,
        endian: Endian,
    ) -> Result<u128, PatternError> {
        let mut buffer = [0_u8; MAX_SCALAR_BYTES];
        let bytes = self.read(offset, size, &mut buffer)?;
        let mut value: u128 = 0;
        match endian {
            Endian::Big => {
                for byte in bytes {
                    value = (value << 8) | u128::from(*byte);
                }
            }
            Endian::Little => {
                for byte in bytes.iter().rev() {
                    value = (value << 8) | u128::from(*byte);
                }
            }
        }
        Ok(value)
    }

    /// Reads `size` bytes at `offset` as a two's-complement signed integer.
    pub(super) fn signed(
        &self,
        offset: u64,
        size: u32,
        endian: Endian,
    ) -> Result<i128, PatternError> {
        let raw = self.unsigned(offset, size, endian)?;
        Ok(sign_extend(raw, size))
    }

    /// Fills the front of `buffer` with `size` bytes from `offset`.
    fn read<'b>(
        &self,
        offset: u64,
        size: u32,
        buffer: &'b mut [u8; MAX_SCALAR_BYTES],
    ) -> Result<&'b [u8], PatternError> {
        let length = usize::try_from(size).map_err(|_| out_of_bounds("size is too large"))?;
        // Not a bound the language can reach — every builtin is sixteen bytes
        // or fewer — and checked anyway, because the alternative is a panic on
        // a slice index if it ever becomes reachable.
        if length > MAX_SCALAR_BYTES {
            return Err(out_of_bounds("read is wider than the widest scalar"));
        }
        let end = offset
            .checked_add(u64::from(size))
            .ok_or_else(|| out_of_bounds("read offset overflows"))?;
        if end > self.len() {
            return Err(out_of_bounds("read extends past the end of the data"));
        }

        let slot = &mut buffer[..length];
        self.source
            .read_exact_at(offset, slot)
            .map_err(|error| source_failed(error.detail()))?;
        Ok(slot)
    }
}

/// Widens an `n`-byte two's-complement value to `i128`.
fn sign_extend(raw: u128, size: u32) -> i128 {
    let bits = size.saturating_mul(8);
    if bits == 0 || bits >= 128 {
        return raw.cast_signed();
    }
    let sign_bit = 1_u128 << (bits - 1);
    if raw & sign_bit == 0 {
        raw.cast_signed()
    } else {
        raw.cast_signed().wrapping_sub(1_i128 << bits)
    }
}

pub(super) fn out_of_bounds(detail: &'static str) -> PatternError {
    PatternError::new(OUT_OF_BOUNDS, Position { line: 0, column: 0 }, detail)
}

/// A read the evaluator had already established was in range, which the source
/// then declined to serve.
///
/// Distinct from [`out_of_bounds`] on purpose: one says the pattern asked for
/// something that is not there, the other says the medium behind the bytes
/// failed. Reporting both under one code would make a disk error look like a
/// malformed pattern.
fn source_failed(detail: &str) -> PatternError {
    PatternError::new(SOURCE_FAILED, Position { line: 0, column: 0 }, detail)
}
