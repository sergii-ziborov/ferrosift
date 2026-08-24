use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use super::delimiter::{DecodeDelimiter, EncodeDelimiter, failed};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Both hex digits for every byte value, resolved at compile time.
///
/// One indexed read per byte instead of two shifts, two masks, and two
/// indexed reads. The table is 512 bytes and stays in L1.
const HEX_PAIRS: [[u8; 2]; 256] = {
    let mut pairs = [[0u8; 2]; 256];
    let mut value = 0usize;
    while value < 256 {
        pairs[value] = [HEX[value >> 4], HEX[value & 0x0f]];
        value += 1;
    }
    pairs
};

/// Contiguous lower-case hex, with no delimiter and no line breaks.
///
/// This is the shape a caller asking for "None" wants, and the shape the
/// specialist crates emit. The general loop below re-decides the delimiter
/// and the line break on every single byte; here there is nothing to decide,
/// so the whole per-byte branch disappears and the bytes go out two at a
/// time.
fn encode_contiguous(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let capacity = input
        .len()
        .checked_mul(2)
        .ok_or(OperationError::OutputLimitExceeded)?;
    ensure_output_fits(capacity, context)?;

    let mut output = Vec::with_capacity(capacity);
    // Cancellation is checked per block rather than per byte: the budget it
    // protects is measured in bytes, and a 4 KiB block is well inside any
    // interval a caller could notice.
    for block in input.chunks(4096) {
        context.ensure_active()?;
        for byte in block.iter().copied() {
            output.extend_from_slice(&HEX_PAIRS[usize::from(byte)]);
        }
    }
    context.ensure_active()?;
    // Every byte written came from HEX_PAIRS, which holds only ASCII, so this
    // cannot fail. It is a validating scan rather than an assertion because
    // the crate forbids unsafe, and the scan is a small fraction of the work
    // it replaces.
    String::from_utf8(output).map_err(|_| failed("encoding.hex.invalid_output"))
}

pub(super) fn encode(
    input: &[u8],
    delimiter: EncodeDelimiter,
    line_size: usize,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    if line_size == 0 && matches!(delimiter, EncodeDelimiter::Suffix("")) {
        return encode_contiguous(input, context);
    }
    let capacity = encoded_len(input.len(), delimiter, line_size)
        .ok_or(OperationError::OutputLimitExceeded)?;
    ensure_output_fits(capacity, context)?;

    let mut output = String::with_capacity(capacity);
    for (index, byte) in input.iter().copied().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        match delimiter {
            EncodeDelimiter::Prefix(prefix) => output.push_str(prefix),
            EncodeDelimiter::PrefixWithComma => output.push_str("0x"),
            EncodeDelimiter::Suffix(_) => {}
        }
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        match delimiter {
            EncodeDelimiter::Suffix(suffix) => output.push_str(suffix),
            EncodeDelimiter::PrefixWithComma => output.push(','),
            EncodeDelimiter::Prefix(_) => {}
        }
        if line_size > 0 && index + 1 < input.len() && (index + 1) % line_size == 0 {
            output.push('\n');
        }
    }
    trim_trailing_delimiter(&mut output, delimiter);
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    delimiter: DecodeDelimiter,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut output = Vec::new();
    match delimiter {
        DecodeDelimiter::Auto => decode_auto(input, &mut output, context)?,
        DecodeDelimiter::Compact => decode_segment(input, &mut output, context)?,
        DecodeDelimiter::Whitespace => {
            for segment in input.split_whitespace() {
                decode_segment(segment, &mut output, context)?;
            }
        }
        DecodeDelimiter::Separated(separator) => {
            decode_separated(input, separator, &mut output, context)?;
        }
        DecodeDelimiter::Prefixed(prefix) => {
            decode_prefixed(input, prefix, &mut output, context)?;
        }
        DecodeDelimiter::PrefixedWithComma => {
            for segment in nonempty_segments(input, ",")? {
                decode_one_prefixed(segment.trim(), "0x", &mut output, context)?;
            }
        }
    }
    if !input.is_empty() && output.is_empty() {
        return Err(failed("encoding.hex.invalid_digit"));
    }
    context.ensure_active()?;
    Ok(output)
}

fn encoded_len(count: usize, delimiter: EncodeDelimiter, line_size: usize) -> Option<usize> {
    if count == 0 {
        return Some(0);
    }
    let (width, trailing) = match delimiter {
        EncodeDelimiter::Suffix(value) => (2_usize.checked_add(value.len())?, value.len()),
        EncodeDelimiter::Prefix(value) => (2_usize.checked_add(value.len())?, 0),
        EncodeDelimiter::PrefixWithComma => (5, 1),
    };
    let body = count.checked_mul(width)?.checked_sub(trailing)?;
    let line_feeds = (count - 1).checked_div(line_size).unwrap_or(0);
    body.checked_add(line_feeds)
}

fn trim_trailing_delimiter(output: &mut String, delimiter: EncodeDelimiter) {
    let length = match delimiter {
        EncodeDelimiter::Suffix(value) => value.len(),
        EncodeDelimiter::PrefixWithComma => 1,
        EncodeDelimiter::Prefix(_) => 0,
    };
    output.truncate(output.len().saturating_sub(length));
}

fn decode_auto(
    input: &str,
    output: &mut Vec<u8>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    let bytes = input.as_bytes();
    let mut start = None;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'0'
            && index + 1 < bytes.len()
            && matches!(bytes[index + 1], b'x' | b'X')
        {
            flush_auto(input, start.take(), index, output, context)?;
            index += 2;
            continue;
        }
        if bytes[index].is_ascii_hexdigit() {
            start.get_or_insert(index);
        } else {
            flush_auto(input, start.take(), index, output, context)?;
        }
        index += 1;
    }
    flush_auto(input, start, bytes.len(), output, context)
}

fn flush_auto(
    input: &str,
    start: Option<usize>,
    end: usize,
    output: &mut Vec<u8>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if let Some(start) = start {
        decode_segment(&input[start..end], output, context)?;
    }
    Ok(())
}

fn decode_separated(
    input: &str,
    separator: &str,
    output: &mut Vec<u8>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    for segment in nonempty_segments(input, separator)? {
        decode_segment(segment.trim(), output, context)?;
    }
    Ok(())
}

fn nonempty_segments<'a>(
    input: &'a str,
    separator: &str,
) -> Result<impl Iterator<Item = &'a str>, OperationError> {
    if input.is_empty() {
        return Ok(input.split(separator).take(0));
    }
    if input
        .split(separator)
        .any(|segment| segment.trim().is_empty())
    {
        return Err(failed("encoding.hex.invalid_delimiter"));
    }
    Ok(input.split(separator).take(usize::MAX))
}

fn decode_prefixed(
    input: &str,
    prefix: &str,
    output: &mut Vec<u8>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if input.is_empty() {
        return Ok(());
    }
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(after_prefix) = remaining.strip_prefix(prefix) else {
            return Err(failed("encoding.hex.invalid_delimiter"));
        };
        if after_prefix.len() < 2 {
            return Err(failed("encoding.hex.odd_length"));
        }
        if !after_prefix.as_bytes()[..2].iter().all(u8::is_ascii) {
            return Err(failed("encoding.hex.invalid_digit"));
        }
        decode_segment(&after_prefix[..2], output, context)?;
        remaining = &after_prefix[2..];
    }
    Ok(())
}

fn decode_one_prefixed(
    input: &str,
    prefix: &str,
    output: &mut Vec<u8>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    let Some(value) = input.strip_prefix(prefix) else {
        return Err(failed("encoding.hex.invalid_delimiter"));
    };
    if value.len() != 2 {
        return Err(failed("encoding.hex.invalid_delimiter"));
    }
    decode_segment(value, output, context)
}

fn decode_segment(
    input: &str,
    output: &mut Vec<u8>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if !input.len().is_multiple_of(2) {
        return Err(failed("encoding.hex.odd_length"));
    }
    for pair in input.as_bytes().chunks_exact(2) {
        if output.len() as u64 >= context.budget().max_output_bytes {
            return Err(OperationError::OutputLimitExceeded);
        }
        let high = nibble(pair[0])?;
        let low = nibble(pair[1])?;
        output.push((high << 4) | low);
        if output.len().is_multiple_of(4096) {
            context.ensure_active()?;
        }
    }
    Ok(())
}

fn nibble(value: u8) -> Result<u8, OperationError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(failed("encoding.hex.invalid_digit")),
    }
}

fn ensure_output_fits(
    output_size: usize,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    let size = u64::try_from(output_size).map_err(|_| OperationError::OutputLimitExceeded)?;
    if size > context.budget().max_output_bytes {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
