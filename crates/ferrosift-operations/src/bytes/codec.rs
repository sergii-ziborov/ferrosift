use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

pub(super) fn take(
    input: &[u8],
    start: i128,
    length: i128,
    apply_to_each_line: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    if apply_to_each_line {
        map_lines(input, start, length, context, |line, start, length| {
            slice_range(line, start, length)
        })
    } else {
        context.ensure_active()?;
        Ok(slice_range(input, start, length))
    }
}

pub(super) fn drop(
    input: &[u8],
    start: i128,
    length: i128,
    apply_to_each_line: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    if apply_to_each_line {
        map_lines(input, start, length, context, |line, start, length| {
            drop_range(line, start, length)
        })
    } else {
        context.ensure_active()?;
        Ok(drop_range(input, start, length))
    }
}

fn map_lines(
    input: &[u8],
    start: i128,
    length: i128,
    context: &OperationContext<'_>,
    map: impl Fn(&[u8], i128, i128) -> Vec<u8>,
) -> Result<Vec<u8>, OperationError> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    for (index, &byte) in input.iter().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        if byte == b'\n' {
            lines.push(core::mem::take(&mut current));
        } else {
            current.push(byte);
        }
    }
    lines.push(current);

    let mut output = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if index % 256 == 0 {
            context.ensure_active()?;
        }
        output.extend(map(line, start, length));
        if index + 1 < lines.len() {
            output.push(b'\n');
        }
    }
    context.ensure_active()?;
    Ok(output)
}

fn slice_range(input: &[u8], start: i128, length: i128) -> Vec<u8> {
    let (start, length) = normalize(input.len(), start, length);
    let end = start.saturating_add(length).min(input.len());
    let start = start.min(input.len());
    input.get(start..end).unwrap_or(&[]).to_vec()
}

fn drop_range(input: &[u8], start: i128, length: i128) -> Vec<u8> {
    let (start, length) = normalize(input.len(), start, length);
    let end = start.saturating_add(length);
    let mut output = Vec::with_capacity(input.len());
    if start < input.len() {
        output.extend_from_slice(&input[..start.min(input.len())]);
        if end < input.len() {
            output.extend_from_slice(&input[end..]);
        }
    } else {
        output.extend_from_slice(input);
    }
    output
}

fn normalize(len: usize, mut start: i128, mut length: i128) -> (usize, usize) {
    let len_i = i128::try_from(len).unwrap_or(i128::MAX);
    if start < 0 {
        start += len_i;
    }
    if length < 0 {
        start += length;
        if start < 0 {
            start += len_i;
            length = start - length;
        } else {
            length = -length;
        }
    }
    let start = if start < 0 {
        0
    } else {
        usize::try_from(start).unwrap_or(len)
    };
    let length = if length < 0 {
        0
    } else {
        usize::try_from(length).unwrap_or(0)
    };
    (start, length)
}
