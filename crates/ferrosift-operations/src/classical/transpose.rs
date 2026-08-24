//! Rail Fence and Caesar Box: ciphers that rearrange rather than
//! substitute.

use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

use super::letters::{from_units, units};

const SMALL_KEY: &str = "cipher.rail_fence.key_too_small";
const LARGE_KEY: &str = "cipher.rail_fence.key_too_large";
const NEGATIVE_OFFSET: &str = "cipher.rail_fence.negative_offset";
const INVALID_HEIGHT: &str = "cipher.caesar_box.invalid_height";
/// Reads the input off a zig-zag of `key` rails.
pub(super) fn rail_fence_encode(
    input: &str,
    key: i128,
    offset: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let source = units(input);
    let (key, offset) = check_rails(key, offset, source.len())?;
    let cycle = (key - 1) * 2;
    let mut rows: Vec<Vec<u16>> = alloc::vec![Vec::new(); key];
    for (position, unit) in source.iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let row = key - 1 - (cycle / 2).abs_diff((position + offset) % cycle);
        rows[row].push(*unit);
    }
    let flattened: Vec<u16> = rows.into_iter().flatten().collect();
    context.ensure_active()?;
    Ok(from_units(&flattened))
}

/// Rebuilds the zig-zag, filling rails in the same order the encoder read them.
///
/// Slots the zig-zag never lands on stay empty rather than becoming NULs: the
/// reference builds a sparse array and joins it, and both a hole and an
/// `undefined` read past the end of the cipher text join as nothing.
pub(super) fn rail_fence_decode(
    input: &str,
    key: i128,
    offset: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let source = units(input);
    let (key, offset) = check_rails(key, offset, source.len())?;
    let cycle = i128::try_from((key - 1) * 2).unwrap_or(1);
    let offset = i128::try_from(offset).unwrap_or(0);
    let mut output: Vec<Option<u16>> = alloc::vec![None; source.len()];
    let mut taken = 0usize;
    for row in 0..key {
        let row = i128::try_from(row).unwrap_or(0);
        for (column, slot) in output.iter_mut().enumerate() {
            if column.is_multiple_of(4096) {
                context.ensure_active()?;
            }
            // JavaScript's `%` keeps the sign, but `x % n === 0` still means
            // exactly "n divides x", so a Euclidean remainder is equivalent.
            let column = i128::try_from(column).unwrap_or(0);
            let forward = (row + column + offset).rem_euclid(cycle) == 0;
            let backward = (row - column - offset).rem_euclid(cycle) == 0;
            if forward || backward {
                *slot = source.get(taken).copied();
                taken += 1;
            }
        }
    }
    let filled: Vec<u16> = output.into_iter().flatten().collect();
    context.ensure_active()?;
    Ok(from_units(&filled))
}

/// Shared rail-fence argument validation.
fn check_rails(key: i128, offset: i128, length: usize) -> Result<(usize, usize), OperationError> {
    if key < 2 {
        return Err(failed(SMALL_KEY));
    }
    if offset < 0 {
        return Err(failed(NEGATIVE_OFFSET));
    }
    let key = usize::try_from(key).map_err(|_| failed(LARGE_KEY))?;
    if key > length {
        return Err(failed(LARGE_KEY));
    }
    let offset = usize::try_from(offset).map_err(|_| failed(NEGATIVE_OFFSET))?;
    Ok((key, offset))
}

/// Writes the input across a fixed-height box and reads it down the columns.
///
/// The reference pads the stripped text with NUL bytes to fill the box and
/// then skips every NUL while reading, so the padding cannot reach the output
/// and the box width never matters. (Its padding loop also re-reads the
/// growing string's length each iteration and so appends only half the gap —
/// invisible for the same reason.) What is left is a plain column read.
///
/// A height below 1 makes the reference divide by zero and loop forever
/// appending padding. There is no output to agree with, so it is rejected.
pub(super) fn caesar_box(
    input: &str,
    height: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if height < 1 {
        return Err(failed(INVALID_HEIGHT));
    }
    let height = usize::try_from(height).map_err(|_| failed(INVALID_HEIGHT))?;
    let stripped: Vec<u16> = units(input)
        .into_iter()
        .filter(|unit| *unit != u16::from(b' '))
        .collect();

    let mut output: Vec<u16> = Vec::with_capacity(stripped.len());
    for row in 0..height {
        let mut column = row;
        while column < stripped.len() {
            if column.is_multiple_of(4096) {
                context.ensure_active()?;
            }
            output.push(stripped[column]);
            column += height;
        }
    }
    context.ensure_active()?;
    Ok(from_units(&output))
}
