//! XOR keyspace enumeration with optional crib filter.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;
use crate::jscompat::escape::parse_escaped_chars;
use crate::xor::codec as xor_codec;

const INVALID_KEY_LENGTH: &str = "logic.xor_brute.invalid_key_length";
const INVALID_SAMPLE: &str = "logic.xor_brute.invalid_sample";

pub(super) struct BruteOptions<'a> {
    pub key_length: i128,
    pub sample_length: i128,
    pub sample_offset: i128,
    pub scheme: &'a str,
    pub null_preserving: bool,
    pub print_key: bool,
    pub output_hex: bool,
    pub crib: &'a str,
}

pub(super) fn brute(
    input: &[u8],
    options: &BruteOptions<'_>,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if !(1..=2).contains(&options.key_length) {
        return Err(failed(INVALID_KEY_LENGTH));
    }
    if options.sample_length < 0 || options.sample_offset < 0 {
        return Err(failed(INVALID_SAMPLE));
    }
    let key_length = usize::try_from(options.key_length).map_err(|_| failed(INVALID_KEY_LENGTH))?;
    let sample_length =
        usize::try_from(options.sample_length).map_err(|_| failed(INVALID_SAMPLE))?;
    let sample_offset =
        usize::try_from(options.sample_offset).map_err(|_| failed(INVALID_SAMPLE))?;
    let end = sample_offset.saturating_add(sample_length).min(input.len());
    let sample = input.get(sample_offset..end).unwrap_or(&[]);
    let crib = parse_escaped_chars(options.crib).to_ascii_lowercase();
    let total = 256_u32.pow(u32::try_from(key_length).unwrap_or(1));
    let mut lines = Vec::new();
    for key_value in 1..total {
        if key_value.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let key = int_to_key(key_value, key_length);
        let result = xor_codec::apply(
            sample,
            &key,
            options.scheme,
            options.null_preserving,
            context,
        )?;
        let decoded = byte_array_to_utf8(&result);
        if !crib.is_empty() && !decoded.to_ascii_lowercase().contains(&crib) {
            continue;
        }
        let mut record = String::new();
        if options.print_key {
            let _ = write!(
                record,
                "Key = {:0width$x}: ",
                key_value,
                width = key_length * 2
            );
        }
        if options.output_hex {
            record.push_str(&to_hex_spaced(&result));
        } else {
            record.push_str(&escape_whitespace(&decoded));
        }
        lines.push(record);
    }
    let output = lines.join("\n");
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}

/// One key from the enumeration, as the numbers XOR reads keys as.
///
/// Doubles because that is what `xor::apply` takes — a toggleString key can
/// hold a number no byte holds, and it works on the number. Nothing here can:
/// every element is a byte of `value` by construction, so the widening is
/// exact and this enumeration reaches none of the behaviour that motivates it.
fn int_to_key(mut value: u32, len: usize) -> Vec<f64> {
    let mut key = alloc::vec![0.0_f64; len];
    for index in (0..len).rev() {
        key[index] = f64::from(value & 0xff);
        value >>= 8;
    }
    key
}

/// Reproduces `Utils.byteArrayToUtf8`: a strict UTF-8 decode, falling back to
/// a Latin-1 char-by-char decode when the bytes are not well-formed UTF-8.
fn byte_array_to_utf8(bytes: &[u8]) -> String {
    match core::str::from_utf8(bytes) {
        Ok(text) => String::from(text),
        Err(_) => bytes.iter().map(|byte| char::from(*byte)).collect(),
    }
}

/// Reproduces `Utils.escapeWhitespace`: control characters in `0x09..=0x10`
/// are shifted into the `U+E000` private-use area so they do not render as
/// whitespace and cannot corrupt the line-delimited report.
fn escape_whitespace(text: &str) -> String {
    text.chars()
        .map(|value| {
            let code = value as u32;
            if (0x09..=0x10).contains(&code) {
                char::from_u32(0xe000 + code).unwrap_or(value)
            } else {
                value
            }
        })
        .collect()
}

fn to_hex_spaced(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push_str(&to_hex_lower(&[*byte]));
    }
    output
}
