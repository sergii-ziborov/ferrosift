use crate::ast::Endian;
use crate::error::{PatternError, Position};

pub(super) const OUT_OF_BOUNDS: &str = "pattern.eval.out_of_bounds";

/// Reads fixed-width scalars out of the evaluated buffer.
///
/// Every read is bounds-checked against the real buffer length, so a pattern
/// can never observe bytes that are not there.
pub(super) struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(super) const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub(super) const fn len(&self) -> u64 {
        self.data.len() as u64
    }

    /// Reads `size` bytes at `offset` as an unsigned integer.
    pub(super) fn unsigned(
        &self,
        offset: u64,
        size: u32,
        endian: Endian,
    ) -> Result<u128, PatternError> {
        let bytes = self.slice(offset, size)?;
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

    fn slice(&self, offset: u64, size: u32) -> Result<&'a [u8], PatternError> {
        let size = u64::from(size);
        let end = offset
            .checked_add(size)
            .ok_or_else(|| out_of_bounds("read offset overflows"))?;
        if end > self.len() {
            return Err(out_of_bounds("read extends past the end of the data"));
        }
        let start = usize::try_from(offset).map_err(|_| out_of_bounds("offset is too large"))?;
        let length = usize::try_from(size).map_err(|_| out_of_bounds("size is too large"))?;
        self.data
            .get(start..start + length)
            .ok_or_else(|| out_of_bounds("read extends past the end of the data"))
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
