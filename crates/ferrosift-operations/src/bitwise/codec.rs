use alloc::{format, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::hex_util;
use crate::jscompat::number::{to_int32, to_uint8};
use crate::jscompat::string::{byte_array_to_utf8, str_to_byte_array};
use crate::key::{INVALID_BYTE_ARRAY, fits_byte_array};

/// The per-byte operators the reference's `bitOp` dispatches to.
#[derive(Clone, Copy)]
pub(super) enum Operator {
    And,
    Or,
    Not,
    Add,
    Sub,
}

impl Operator {
    /// One per-byte step, on the numbers the reference has rather than on
    /// bytes.
    ///
    /// The key is whatever `Utils.convertToByteArray` produced, which for a
    /// Decimal field is any number at all, and `bitOp` does not coerce it
    /// before applying these. The split below is the reference's own and it
    /// matters: the three bitwise operators convert both sides with `ToInt32`,
    /// so a `NaN` key acts as zero, while `add` and `sub` are plain arithmetic
    /// and carry a `NaN` through into the result.
    ///
    /// Masking the key to a byte first would agree with all five whenever the
    /// key is a number — every one of them is congruent modulo 256 in the key —
    /// which is why nothing caught it. It disagrees on `NaN` for exactly two.
    fn apply(self, operand: u8, key: f64) -> f64 {
        let value = f64::from(operand);
        match self {
            Self::And => f64::from(to_int32(value) & to_int32(key)),
            Self::Or => f64::from(to_int32(value) | to_int32(key)),
            Self::Not => f64::from(!to_int32(value) & 0xff),
            // `(o + k) % 256`, where JavaScript's `%` keeps the sign of the
            // dividend and so does Rust's.
            Self::Add => (value + key) % 256.0,
            // `r < 0 ? 256 + r : r`, and a `NaN` compares false, so it is
            // returned rather than corrected.
            Self::Sub => {
                let result = value - key;
                if result < 0.0 { 256.0 + result } else { result }
            }
        }
    }
}

/// Applies an operator byte-wise against a repeating key.
///
/// An empty key becomes a single zero, matching the reference, so AND with no
/// key zeroes the input rather than passing it through.
///
/// Becoming bytes happens once, at the end, because that is where the reference
/// does it: `bitOp` pushes raw numbers and the dish decides afterwards whether
/// the array it was handed is a byte array at all. Which is why a result out of
/// range fails rather than wrapping — see [`fits_byte_array`]. A key of `300`
/// therefore fails an `OR` and succeeds an `ADD`, because `o | 300` keeps the
/// ninth bit and `(o + 300) % 256` does not.
pub(super) fn bit_op(
    input: &[u8],
    key: &[f64],
    operator: Operator,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let zero = [0.0_f64];
    let key = if key.is_empty() { &zero[..] } else { key };
    let mut output = Vec::with_capacity(input.len());
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let result = operator.apply(*byte, key[index % key.len()]);
        if !fits_byte_array(result) {
            return Err(failed(INVALID_BYTE_ARRAY));
        }
        output.push(to_uint8(result));
    }
    context.ensure_active()?;
    Ok(output)
}

/// JavaScript's shift operators mask the count to five bits after converting
/// it to a 32-bit integer, so a shift of 8 is a shift of 8 but a shift of 32
/// is no shift at all.
const fn js_shift_count(amount: i128) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "this is exactly ToInt32 followed by ToUint32, which JavaScript applies here"
    )]
    let masked = (amount as i32) as u32 & 31;
    masked
}

/// The bound `Bit shift left` declares on its amount.
///
/// Unlike `Bit shift right`, that operation constrains the argument to 0..=7
/// and the reference rejects anything outside it before `run` is ever reached.
/// The two therefore behave differently for an amount of 8: a shift right
/// masks it to a shift of 8, a shift left refuses.
const MAX_SHIFT_LEFT: i128 = 7;
const INVALID_AMOUNT: &str = "logic.shift.left.invalid_amount";

/// `(b << amount) & 0xff`, for an amount the reference would accept.
pub(super) fn shift_left(
    input: &[u8],
    amount: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if !(0..=MAX_SHIFT_LEFT).contains(&amount) {
        return Err(crate::failure::failed(INVALID_AMOUNT));
    }
    let count = js_shift_count(amount);
    let mut output = Vec::with_capacity(input.len());
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let shifted = i32::from(*byte).wrapping_shl(count);
        output.push(u8::try_from(shifted & 0xff).unwrap_or(0));
    }
    context.ensure_active()?;
    Ok(output)
}

/// `(b >>> amount) ^ (b & mask)`, where an arithmetic shift keeps the sign bit.
pub(super) fn shift_right(
    input: &[u8],
    amount: i128,
    arithmetic: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let count = js_shift_count(amount);
    let mask = u32::from(arithmetic) * 0x80;
    let mut output = Vec::with_capacity(input.len());
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let value = u32::from(*byte);
        let shifted = (value >> count) ^ (value & mask);
        output.push(u8::try_from(shifted & 0xff).unwrap_or(0));
    }
    context.ensure_active()?;
    Ok(output)
}

/// Which direction `rotate` and `rotate_carry` turn.
#[derive(Clone, Copy)]
pub(super) enum Direction {
    Left,
    Right,
}

/// Rotates each byte independently.
///
/// The reference applies a single-bit rotation `amount` times, so a negative
/// or zero amount leaves the input alone and the period is eight.
pub(super) fn rotate(
    input: &[u8],
    direction: Direction,
    amount: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let steps = u32::try_from(amount.max(0) % 8).unwrap_or(0);
    let mut output = Vec::with_capacity(input.len());
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        output.push(match direction {
            Direction::Left => byte.rotate_left(steps),
            Direction::Right => byte.rotate_right(steps),
        });
    }
    context.ensure_active()?;
    Ok(output)
}

/// Rotates the buffer as one long bit string, carrying across byte boundaries.
///
/// The two directions disagree on empty input, and the reference's array
/// indexing is why. Rotating right finishes with `result[0] |= carry`, which
/// on an empty array *creates* index 0 and yields a single zero byte; rotating
/// left finishes with `result[length - 1] |= carry`, which writes to index
/// `-1`, a plain property that leaves the array empty.
pub(super) fn rotate_carry(
    input: &[u8],
    direction: Direction,
    amount: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if input.is_empty() {
        return Ok(match direction {
            Direction::Right => alloc::vec![0u8],
            Direction::Left => Vec::new(),
        });
    }
    let amount = amount % 8;
    // `Math.pow(2, amount) - 1` is fractional for a negative amount, and the
    // shift that consumes it converts to zero.
    let mask = if amount < 0 {
        0u32
    } else {
        (1u32 << amount) - 1
    };
    let shift = js_shift_count(amount);
    let inverse = js_shift_count(8 - amount);
    let mut output = alloc::vec![0u8; input.len()];
    let mut carry = 0u32;
    match direction {
        Direction::Right => {
            for (index, byte) in input.iter().enumerate() {
                if index.is_multiple_of(4096) {
                    context.ensure_active()?;
                }
                let old = u32::from(*byte);
                output[index] = u8::try_from(((old >> shift) | carry) & 0xff).unwrap_or(0);
                carry = (old & mask) << inverse;
            }
            output[0] |= u8::try_from(carry & 0xff).unwrap_or(0);
        }
        Direction::Left => {
            for index in (0..input.len()).rev() {
                if index.is_multiple_of(4096) {
                    context.ensure_active()?;
                }
                let old = u32::from(input[index]);
                output[index] = u8::try_from(((old << shift) | carry) & 0xff).unwrap_or(0);
                carry = (old >> inverse) & mask;
            }
            let last = output.len() - 1;
            output[last] |= u8::try_from(carry & 0xff).unwrap_or(0);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// The ROR13 rolling hash used by shellcode API resolution.
pub(super) fn ror13(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut hash: u32 = 0;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        hash = hash.rotate_right(13).wrapping_add(u32::from(*byte));
    }
    context.ensure_active()?;
    Ok(format!("0x{hash:08X}"))
}

const INVALID_WORD_LENGTH: &str = "data.swap_endianness.invalid_word_length";
const INVALID_FORMAT: &str = "data.swap_endianness.invalid_format";

/// Reverses the byte order within each fixed-width word.
pub(super) fn swap_endianness(
    input: &str,
    format: &str,
    word_length: i128,
    pad: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if word_length <= 0 {
        return Err(crate::failure::failed(INVALID_WORD_LENGTH));
    }
    let width =
        usize::try_from(word_length).map_err(|_| crate::failure::failed(INVALID_WORD_LENGTH))?;
    let data = match format {
        "Hex" => hex_util::from_hex_auto(input),
        "Raw" => str_to_byte_array(input),
        _ => return Err(crate::failure::failed(INVALID_FORMAT)),
    };

    let mut result = Vec::with_capacity(data.len() + width);
    for (index, chunk) in data.chunks(width).enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        let mut word = chunk.to_vec();
        if pad {
            word.resize(width, 0);
        }
        word.reverse();
        result.extend_from_slice(&word);
    }

    context.ensure_active()?;
    Ok(match format {
        "Hex" => result
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" "),
        _ => byte_array_to_utf8(&result),
    })
}
