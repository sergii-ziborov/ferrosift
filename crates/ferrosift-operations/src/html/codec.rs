use alloc::{
    format,
    string::{String, ToString},
};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

use super::entities::{named_code, named_entity};

const INVALID_MODE: &str = "encoding.html.invalid_mode";

pub(super) fn encode(
    input: &str,
    convert_all: bool,
    mode: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let numeric = mode == "Numeric entities";
    let hexa = mode == "Hex entities";
    if !numeric && !hexa && mode != "Named entities" {
        return Err(failed(INVALID_MODE));
    }
    let mut output = String::with_capacity(input.len());
    for (index, ch) in input.chars().enumerate() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        let code = ch as u32;
        emit_entity(
            &mut output,
            code,
            named_entity(code),
            convert_all,
            numeric,
            hexa,
        );
    }
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}

fn emit_entity(
    output: &mut String,
    code: u32,
    named: Option<&str>,
    convert_all: bool,
    numeric: bool,
    hexa: bool,
) {
    if convert_all && numeric {
        push_numeric(output, code);
    } else if convert_all && hexa {
        push_hex(output, code);
    } else if convert_all {
        if let Some(name) = named {
            push_named(output, name);
        } else {
            push_numeric(output, code);
        }
    } else if numeric {
        if code > 255 || named.is_some() {
            push_numeric(output, code);
        } else if let Some(ch) = char::from_u32(code) {
            output.push(ch);
        }
    } else if hexa {
        if code > 255 || named.is_some() {
            push_hex(output, code);
        } else if let Some(ch) = char::from_u32(code) {
            output.push(ch);
        }
    } else if let Some(name) = named {
        push_named(output, name);
    } else if code > 255 {
        push_numeric(output, code);
    } else if let Some(ch) = char::from_u32(code) {
        output.push(ch);
    }
}

fn push_named(output: &mut String, name: &str) {
    output.push('&');
    output.push_str(name);
    output.push(';');
}

fn push_numeric(output: &mut String, code: u32) {
    output.push_str("&#");
    output.push_str(&code.to_string());
    output.push(';');
}

fn push_hex(output: &mut String, code: u32) {
    output.push_str("&#x");
    output.push_str(&format_hex_code(code));
    output.push(';');
}

fn format_hex_code(code: u32) -> String {
    if let Ok(byte) = u8::try_from(code) {
        to_hex_lower(&[byte])
    } else {
        let mut text = format!("{code:x}");
        if !text.len().is_multiple_of(2) {
            text.insert(0, '0');
        }
        text
    }
}

pub(super) fn decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::with_capacity(input.len());
    let mut index = 0_usize;
    while index < input.len() {
        if index.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        if !input[index..].starts_with('&') {
            output.push(next_char(input, &mut index));
            continue;
        }
        let start = index;
        index += 1;
        let body_start = index;
        let mut found_semi = false;
        while index < input.len() {
            let ch = next_char(input, &mut index);
            if ch == ';' {
                found_semi = true;
                break;
            }
            if index - body_start > 20 {
                break;
            }
        }
        if !found_semi {
            index = start;
            output.push(next_char(input, &mut index));
            continue;
        }
        let body = &input[body_start..index - 1];
        if let Some(code) = decode_entity_body(body) {
            if let Some(ch) = char::from_u32(code) {
                output.push(ch);
            }
        } else {
            output.push_str(&input[start..index]);
        }
    }
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}

fn decode_entity_body(body: &str) -> Option<u32> {
    if let Some(name_code) = named_code(body) {
        return Some(name_code);
    }
    let digits = body.strip_prefix('#')?;
    if let Some(hex) = digits
        .strip_prefix('x')
        .or_else(|| digits.strip_prefix('X'))
    {
        if (2..=8).contains(&hex.len()) {
            return u32::from_str_radix(hex, 16).ok();
        }
        return None;
    }
    if (1..=6).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit()) {
        return digits.parse().ok();
    }
    None
}

fn next_char(input: &str, index: &mut usize) -> char {
    let ch = input[*index..].chars().next().unwrap_or('\0');
    *index += ch.len_utf8();
    ch
}
