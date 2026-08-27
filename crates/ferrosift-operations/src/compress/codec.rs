//! DEFLATE, and the two containers built on it.
//!
//! `miniz_oxide` is `no_std`, which is why this half of the compression pack
//! reaches bare metal and the bzip2 half beside it does not.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};
use miniz_oxide::deflate::{CompressionLevel, compress_to_vec, compress_to_vec_zlib};
use miniz_oxide::inflate::{
    DecompressError, TINFLStatus, decompress_to_vec_with_limit, decompress_to_vec_zlib_with_limit,
};

use super::limits::{ensure_fits, output_limit};
use crate::crc32::crc32;
use crate::failure::failed;

const INVALID_GZIP: &str = "compression.gzip.invalid";
const INVALID_ZLIB: &str = "compression.zlib.invalid";
const INVALID_RAW: &str = "compression.raw.invalid";
const INVALID_LEVEL: &str = "compression.invalid_level";

pub(super) fn gunzip(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let deflate = skip_gzip_header(input)?;
    let output = inflated(
        decompress_to_vec_with_limit(deflate, output_limit(context)),
        INVALID_GZIP,
    )?;
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn gzip(
    input: &[u8],
    compression_type: &str,
    filename: &str,
    comment: &str,
    include_checksum: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let level = compression_level(compression_type)?;
    let body = compress_to_vec(input, level);
    let mut flags = 0_u8;
    if !filename.is_empty() {
        flags |= 0x08;
    }
    if !comment.is_empty() {
        flags |= 0x10;
    }
    if include_checksum {
        flags |= 0x02;
    }
    let mut output = Vec::with_capacity(18 + body.len() + filename.len() + comment.len());
    output.extend_from_slice(&[0x1f, 0x8b, 0x08, flags, 0, 0, 0, 0, 0, 0xff]);
    if !filename.is_empty() {
        output.extend_from_slice(filename.as_bytes());
        output.push(0);
    }
    if !comment.is_empty() {
        output.extend_from_slice(comment.as_bytes());
        output.push(0);
    }
    if include_checksum {
        let header_crc = crc32(&output);
        let header_crc16 = u16::try_from(header_crc & 0xffff).unwrap_or(0);
        output.extend_from_slice(&header_crc16.to_le_bytes());
    }
    output.extend_from_slice(&body);
    output.extend_from_slice(&crc32(input).to_le_bytes());
    let isize = u32::try_from(input.len() & 0xffff_ffff).unwrap_or(u32::MAX);
    output.extend_from_slice(&isize.to_le_bytes());
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn zlib_deflate(
    input: &[u8],
    compression_type: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let level = compression_level(compression_type)?;
    let output = compress_to_vec_zlib(input, level);
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn zlib_inflate(
    input: &[u8],
    start_index: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if start_index < 0 {
        return Err(failed(INVALID_ZLIB));
    }
    let start = usize::try_from(start_index).map_err(|_| failed(INVALID_ZLIB))?;
    let slice = input.get(start..).ok_or_else(|| failed(INVALID_ZLIB))?;
    let output = inflated(
        decompress_to_vec_zlib_with_limit(slice, output_limit(context)),
        INVALID_ZLIB,
    )?;
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn raw_deflate(
    input: &[u8],
    compression_type: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let level = compression_level(compression_type)?;
    let output = compress_to_vec(input, level);
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn raw_inflate(
    input: &[u8],
    start_index: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if start_index < 0 {
        return Err(failed(INVALID_RAW));
    }
    let start = usize::try_from(start_index).map_err(|_| failed(INVALID_RAW))?;
    let slice = input.get(start..).ok_or_else(|| failed(INVALID_RAW))?;
    let output = inflated(
        decompress_to_vec_with_limit(slice, output_limit(context)),
        INVALID_RAW,
    )?;
    ensure_fits(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

fn compression_level(token: &str) -> Result<u8, OperationError> {
    // CyberChef compression type names map onto zlibjs levels; Default/Dynamic use
    // miniz default compression which is sufficient for interoperable inflate.
    match token {
        "Dynamic Huffman Coding" | "Fixed Huffman Coding" | "None (Store)" => {
            Ok(CompressionLevel::DefaultLevel as u8)
        }
        _ => Err(failed(INVALID_LEVEL)),
    }
}

/// Reads an inflate result, keeping "too large" and "malformed" apart.
///
/// Bounding the decompressor is what stops a small input from allocating
/// gigabytes before anything gets to look at it, but it moves the size refusal
/// from `ensure_fits` into `miniz`'s own error path, and `miniz` reports it the
/// same way it reports a truncated stream: as an error. Only the status tells
/// the two apart, so a bomb keeps reporting `OutputLimitExceeded` rather than
/// being relabelled as invalid input, and the pinned failure codes for genuinely
/// broken streams stay where they were.
fn inflated(
    result: Result<Vec<u8>, DecompressError>,
    code: &'static str,
) -> Result<Vec<u8>, OperationError> {
    result.map_err(|error| match error.status {
        TINFLStatus::HasMoreOutput => OperationError::OutputLimitExceeded,
        _ => failed(code),
    })
}

pub(super) fn skip_gzip_header(input: &[u8]) -> Result<&[u8], OperationError> {
    if input.len() < 10 || input[0] != 0x1f || input[1] != 0x8b || input[2] != 0x08 {
        return Err(failed(INVALID_GZIP));
    }
    let flags = input[3];
    let mut index = 10_usize;
    if flags & 0x04 != 0 {
        if index + 2 > input.len() {
            return Err(failed(INVALID_GZIP));
        }
        let xlen = usize::from(u16::from_le_bytes([input[index], input[index + 1]]));
        index = index
            .checked_add(2 + xlen)
            .ok_or_else(|| failed(INVALID_GZIP))?;
    }
    if flags & 0x08 != 0 {
        index = skip_c_string(input, index)?;
    }
    if flags & 0x10 != 0 {
        index = skip_c_string(input, index)?;
    }
    if flags & 0x02 != 0 {
        index = index.checked_add(2).ok_or_else(|| failed(INVALID_GZIP))?;
    }
    if index + 8 > input.len() {
        return Err(failed(INVALID_GZIP));
    }
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
    Err(failed(INVALID_GZIP))
}
