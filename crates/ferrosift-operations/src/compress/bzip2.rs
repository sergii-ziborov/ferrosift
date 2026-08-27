//! Bzip2, kept apart from the DEFLATE codec beside it.
//!
//! Not because the algorithms are unrelated but because their portability is:
//! `miniz_oxide` is `no_std` and this reaches `thiserror`, which is not. One
//! file each is what lets `portable-full` mean what it says.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use super::limits::{ensure_fits, output_limit};
use crate::failure::failed;

const INVALID_BZIP2: &str = "compression.bzip2.invalid";
const EMPTY_BZIP2: &str = "compression.bzip2.empty_input";
const INVALID_BLOCK: &str = "compression.bzip2.invalid_block_size";

pub(super) fn bzip2_compress(
    input: &[u8],
    block_size: i128,
    _work_factor: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if input.is_empty() {
        return Err(failed(EMPTY_BZIP2));
    }
    if !(1..=9).contains(&block_size) {
        return Err(failed(INVALID_BLOCK));
    }
    let level = oxiarc_bzip2::CompressionLevel::new(u8::try_from(block_size).unwrap_or(9));
    let output = oxiarc_bzip2::compress(input, level).map_err(|_| failed(INVALID_BZIP2))?;
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn bzip2_decompress(
    input: &[u8],
    _low_memory: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if input.is_empty() {
        return Err(failed(EMPTY_BZIP2));
    }
    // Bounded before it allocates, for the same reason the inflate paths are:
    // a small compressed input can ask for an unbounded output, and checking
    // afterwards means the allocation already happened.
    let output = oxiarc_bzip2::decompress_with_limit(input, output_limit(context))
        .map_err(|_| failed(INVALID_BZIP2))?;
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}
