use alloc::{format, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const MAX_WIDTH: i128 = 65_536;
const INVALID_WIDTH: &str = "encoding.hexdump.invalid_width";

pub(super) fn encode(
    input: &[u8],
    width: i128,
    upper_case: bool,
    include_final_length: bool,
    unix_format: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    if !(1..=MAX_WIDTH).contains(&width) {
        return Err(failed(INVALID_WIDTH));
    }
    let width = usize::try_from(width).map_err(|_| failed(INVALID_WIDTH))?;
    let line_capacity = 14 + width * 4;
    let lines = input
        .len()
        .div_ceil(width)
        .max(usize::from(!input.is_empty()));
    let capacity = lines
        .saturating_mul(line_capacity + 1)
        .saturating_add(if include_final_length { 9 } else { 0 });
    if u64::try_from(capacity).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut output = String::with_capacity(capacity);
    if input.is_empty() {
        context.ensure_active()?;
        return Ok(output);
    }

    let mut offset = 0_usize;
    while offset < input.len() {
        if offset.is_multiple_of(width * 64) {
            context.ensure_active()?;
        }
        let end = (offset + width).min(input.len());
        let chunk = &input[offset..end];
        let mut line_no = format_hex(offset as u64, 8);
        let mut hex = String::with_capacity(width * 3);
        for (index, byte) in chunk.iter().enumerate() {
            if index > 0 {
                hex.push(' ');
            }
            hex.push_str(&format_hex(u64::from(*byte), 2));
        }
        while hex.len() < width * 3 {
            hex.push(' ');
        }
        let mut ascii = String::with_capacity(chunk.len());
        for &byte in chunk {
            ascii.push(printable_char(byte, unix_format));
        }
        if upper_case {
            line_no = line_no.to_ascii_uppercase();
            hex = hex.to_ascii_uppercase();
        }
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&line_no);
        output.push_str("  ");
        output.push_str(&hex);
        output.push_str(" |");
        output.push_str(&ascii);
        output.push('|');
        offset = end;
        if include_final_length && offset == input.len() {
            // The reference pushes the final-length line as raw lowercase hex
            // after the per-line upper-casing, so it stays lowercase even when
            // upper-case mode is on.
            output.push('\n');
            output.push_str(&format_hex(offset as u64, 8));
        }
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut output = Vec::new();
    for (line_index, line) in input.lines().enumerate() {
        if line_index % 256 == 0 {
            context.ensure_active()?;
        }
        if let Some(bytes) = extract_hex_bytes(line) {
            for pair in bytes.chunks(2) {
                if pair.len() == 2 {
                    let text = core::str::from_utf8(pair)
                        .map_err(|_| failed("encoding.hexdump.invalid_digit"))?;
                    let byte = u8::from_str_radix(text, 16)
                        .map_err(|_| failed("encoding.hexdump.invalid_digit"))?;
                    output.push(byte);
                }
            }
        }
    }
    context.ensure_active()?;
    Ok(output)
}

fn extract_hex_bytes(line: &str) -> Option<Vec<u8>> {
    // Mirrors FromHexdump's capture group: optional offset, then hex byte runs.
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let mut chars = trimmed.chars().peekable();
    // Skip optional offset: 4-16 hex digits with optional trailing h/:
    let mut offset_digits = 0_usize;
    while let Some(value) = chars.peek().copied() {
        if value.is_ascii_hexdigit() {
            offset_digits += 1;
            chars.next();
        } else {
            break;
        }
    }
    if (4..=16).contains(&offset_digits) {
        if matches!(chars.peek().copied(), Some('h' | 'H' | ':')) {
            chars.next();
        }
    } else {
        // Not an offset; restart from full line for pure hex dumps.
        chars = trimmed.chars().peekable();
    }
    // Require whitespace before the hex payload when an offset was present.
    let mut saw_ws = false;
    while matches!(chars.peek().copied(), Some(' ' | '\t')) {
        saw_ws = true;
        chars.next();
    }
    if offset_digits >= 4 && !saw_ws && offset_digits <= 16 {
        // Offset without payload.
        return None;
    }

    let rest: String = chars.collect();
    let mut hex = Vec::new();
    let mut index = 0_usize;
    let bytes = rest.as_bytes();
    while index < bytes.len() {
        let value = bytes[index];
        if value.is_ascii_hexdigit() {
            hex.push(value.to_ascii_lowercase());
            index += 1;
            if index < bytes.len() && bytes[index].is_ascii_hexdigit() {
                hex.push(bytes[index].to_ascii_lowercase());
                index += 1;
            } else {
                // Lone nibble ends the hex run.
                break;
            }
            if index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'-') {
                index += 1;
                continue;
            }
            if index < bytes.len() && bytes[index] == b'\t' {
                break;
            }
            continue;
        }
        if value == b'|' || value == b' ' || value == b'\t' {
            break;
        }
        index += 1;
    }
    if hex.is_empty() { None } else { Some(hex) }
}

fn format_hex(value: u64, width: usize) -> String {
    let mut text = format!("{value:x}");
    while text.len() < width {
        text.insert(0, '0');
    }
    text
}

fn printable_char(byte: u8, unix_format: bool) -> char {
    if unix_format {
        if (0x20..=0x7e).contains(&byte) {
            char::from(byte)
        } else {
            '.'
        }
    } else if is_non_printable_latin1(byte) {
        '.'
    } else {
        char::from_u32(u32::from(byte)).unwrap_or('.')
    }
}

fn is_non_printable_latin1(byte: u8) -> bool {
    matches!(
        byte,
        0x00..=0x08
            | 0x0B..=0x0C
            | 0x0E..=0x1F
            | 0x7F..=0x9F
            | 0xAD
    ) || matches!(byte, 0x09 | 0x0A | 0x0D)
}
