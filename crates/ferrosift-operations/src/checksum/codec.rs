use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The Adler-32 modulus: the largest prime below 65536.
const ADLER_MODULUS: u128 = 65521;

/// Adler-32: two running sums, reduced modulo 65521 only at the end.
///
/// The reference accumulates in JavaScript numbers and reduces once after the
/// loop, so the intermediate sums are exact up to 2^53. Accumulating in a
/// 128-bit integer matches that for any input a budget will admit.
pub(super) fn adler32(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut a: u128 = 1;
    let mut b: u128 = 0;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        a += u128::from(*byte);
        b += a;
    }
    let a = a % ADLER_MODULUS;
    let b = b % ADLER_MODULUS;
    context.ensure_active()?;
    Ok(format!("{:08x}", (b << 16) | a))
}

/// Which Fletcher width to compute.
#[derive(Clone, Copy)]
pub(super) enum Fletcher {
    Eight,
    Sixteen,
    ThirtyTwo,
    SixtyFour,
}

/// Fletcher checksums at four widths.
///
/// Each reduces modulo one less than its word maximum — `0xf`, `0xff`,
/// `0xffff`, `0xffffffff` — not the maximum itself.
pub(super) fn fletcher(
    input: &[u8],
    width: Fletcher,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    match width {
        Fletcher::Eight => Ok(narrow(input, 0xf, 4, context)?),
        Fletcher::Sixteen => Ok(narrow(input, 0xff, 8, context)?),
        Fletcher::ThirtyTwo => fletcher32(input, context),
        Fletcher::SixtyFour => fletcher64(input, context),
    }
}

/// The byte-wise Fletcher variants, which differ only in modulus and width.
fn narrow(
    input: &[u8],
    modulus: u64,
    shift: u32,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let mut a: u64 = 0;
    let mut b: u64 = 0;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        a = (a + u64::from(*byte)) % modulus;
        b = (b + a) % modulus;
    }
    let digits = (shift / 4) * 2;
    Ok(format!(
        "{:0width$x}",
        (b << shift) | a,
        width = digits as usize
    ))
}

/// Fletcher-32 reads little-endian 16-bit words, then folds a lone trailing
/// byte in on its own.
fn fletcher32(input: &[u8], context: &OperationContext<'_>) -> Result<String, OperationError> {
    let mut a: u64 = 0;
    let mut b: u64 = 0;
    for (index, pair) in input.chunks_exact(2).enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let word = u64::from(u16::from_le_bytes([pair[0], pair[1]]));
        a = (a + word) % 0xffff;
        b = (b + a) % 0xffff;
    }
    if let [last] = input.chunks_exact(2).remainder() {
        a = (a + u64::from(*last)) % 0xffff;
        b = (b + a) % 0xffff;
    }
    Ok(format!("{:08x}", (b << 16) | a))
}

/// Fletcher-64 reads little-endian 32-bit words, and assembles its trailing
/// partial word by walking backwards from the end — so a two-byte tail is read
/// as `last << 8 | second-to-last`, the reverse of the little-endian order used
/// for the full words.
fn fletcher64(input: &[u8], context: &OperationContext<'_>) -> Result<String, OperationError> {
    let mut a: u64 = 0;
    let mut b: u64 = 0;
    for (index, quad) in input.chunks_exact(4).enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let word = u64::from(u32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]));
        a = (a + word) % 0xffff_ffff;
        b = (b + a) % 0xffff_ffff;
    }
    let remainder = input.chunks_exact(4).remainder();
    if !remainder.is_empty() {
        let mut last: u64 = 0;
        for byte in remainder.iter().rev() {
            last = (last << 8) | u64::from(*byte);
        }
        a = (a + last) % 0xffff_ffff;
        b = (b + a) % 0xffff_ffff;
    }
    Ok(format!("{b:08x}{a:08x}"))
}

/// The TCP/IP header checksum.
///
/// The reference folds the accumulator once and then subtracts from `0xffff`,
/// which can go negative when one fold is not enough to reduce it. The
/// negative is then rendered with a minus sign rather than wrapped, and that
/// is reproduced here.
pub(super) fn tcp_ip(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut sum: i64 = 0;
    for (index, byte) in input.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if index.is_multiple_of(2) {
            sum += i64::from(*byte) << 8;
        } else {
            sum += i64::from(*byte);
        }
    }
    let folded = (sum >> 16) + (sum & 0xffff);
    let value = 0xffff - folded;
    context.ensure_active()?;
    Ok(if value < 0 {
        format!("-{:x}", -value)
    } else {
        format!("{value:02x}")
    })
}

const INVALID_BLOCK_SIZE: &str = "checksum.xor.invalid_block_size";

/// XORs the input together in fixed-size blocks.
///
/// A short final block is padded with zeros rather than skipped: the reference
/// reads past its end, and `x ^ undefined` is `x`.
pub(super) fn xor_checksum(
    input: &[u8],
    block_size: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if block_size <= 0 {
        return Err(failed(INVALID_BLOCK_SIZE));
    }
    let width = usize::try_from(block_size).map_err(|_| failed(INVALID_BLOCK_SIZE))?;
    let mut accumulator = alloc::vec![0u8; width];
    for (index, block) in input.chunks(width).enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        for (slot, byte) in accumulator.iter_mut().zip(block) {
            *slot ^= byte;
        }
    }
    context.ensure_active()?;
    Ok(crate::hex_util::to_hex_lower(&accumulator))
}

const INVALID_RADIX: &str = "checksum.luhn.invalid_radix";
const INVALID_DIGIT: &str = "checksum.luhn.invalid_digit";

/// The Luhn checksum, generalised to any even radix from 2 to 36.
///
/// The report is three lines, and the check digit is computed from the input
/// with a zero appended — so it is the digit that would make the whole string
/// validate.
pub(super) fn luhn(
    input: &str,
    radix: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if input.is_empty() {
        return Ok(String::new());
    }
    if !(2..=36).contains(&radix) || radix % 2 != 0 {
        return Err(failed(INVALID_RADIX));
    }
    let radix = u32::try_from(radix).map_err(|_| failed(INVALID_RADIX))?;
    let sum = luhn_sum(input, radix, context)?;
    let with_zero = luhn_sum(&format!("{input}0"), radix, context)?;
    let check = if with_zero == 0 { 0 } else { radix - with_zero };
    let checksum = digit_string(sum, radix);
    let check_digit = digit_string(check, radix);
    context.ensure_active()?;
    Ok(format!(
        "Checksum: {checksum}\nCheckdigit: {check_digit}\nLuhn Validated String: {input}{check_digit}"
    ))
}

/// Sums the digits, doubling every second one from the right.
fn luhn_sum(
    input: &str,
    radix: u32,
    context: &OperationContext<'_>,
) -> Result<u32, OperationError> {
    let mut total: u64 = 0;
    let mut double = false;
    for (index, character) in input.chars().rev().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let mut value = character
            .to_digit(radix)
            .ok_or_else(|| failed(INVALID_DIGIT))?;
        if double {
            value *= 2;
            value = value / radix + value % radix;
        }
        double = !double;
        total += u64::from(value);
    }
    u32::try_from(total % u64::from(radix)).map_err(|_| failed(INVALID_RADIX))
}

/// Renders one value as a digit in the given radix, as `Number.toString` does.
fn digit_string(value: u32, radix: u32) -> String {
    let mut digits = Vec::new();
    let mut value = value;
    if value == 0 {
        return "0".to_string();
    }
    while value > 0 {
        let digit = value % radix;
        digits.push(char::from_digit(digit, radix).unwrap_or('0'));
        value /= radix;
    }
    digits.iter().rev().collect()
}
