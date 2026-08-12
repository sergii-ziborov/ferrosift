use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};
use miniz_oxide::inflate::decompress_to_vec;

use crate::failure::failed;

const INVALID: &str = "compression.gunzip.invalid";

pub(super) fn decompress(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let deflate = skip_gzip_header(input)?;
    let output = decompress_to_vec(deflate).map_err(|_| failed(INVALID))?;
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}

fn skip_gzip_header(input: &[u8]) -> Result<&[u8], OperationError> {
    if input.len() < 10 || input[0] != 0x1f || input[1] != 0x8b || input[2] != 0x08 {
        return Err(failed(INVALID));
    }
    let flags = input[3];
    let mut index = 10_usize;
    // FEXTRA
    if flags & 0x04 != 0 {
        if index + 2 > input.len() {
            return Err(failed(INVALID));
        }
        let xlen = usize::from(u16::from_le_bytes([input[index], input[index + 1]]));
        index = index.checked_add(2 + xlen).ok_or_else(|| failed(INVALID))?;
    }
    // FNAME
    if flags & 0x08 != 0 {
        index = skip_c_string(input, index)?;
    }
    // FCOMMENT
    if flags & 0x10 != 0 {
        index = skip_c_string(input, index)?;
    }
    // FHCRC
    if flags & 0x02 != 0 {
        index = index.checked_add(2).ok_or_else(|| failed(INVALID))?;
    }
    if index + 8 > input.len() {
        return Err(failed(INVALID));
    }
    // Drop trailing CRC32 + ISIZE.
    Ok(&input[index..input.len() - 8])
}

fn skip_c_string(input: &[u8], mut index: usize) -> Result<usize, OperationError> {
    while index < input.len() {
        let value = input[index];
        index += 1;
        if value == 0 {
            return Ok(index);
        }
    }
    Err(failed(INVALID))
}
