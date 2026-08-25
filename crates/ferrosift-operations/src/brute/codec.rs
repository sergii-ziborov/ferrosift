use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::jscompat::string::byte_array_to_utf8;

/// Which characters a ROT13 brute force rotates.
#[derive(Clone, Copy)]
pub(super) struct Rot13Options {
    pub lower: bool,
    pub upper: bool,
    pub numbers: bool,
}

/// The sample window both operations take before rotating.
#[derive(Clone, Copy)]
pub(super) struct Sample {
    pub offset: usize,
    pub length: usize,
}

impl Sample {
    /// The reference slices with `slice(offset, offset + length)`, which
    /// clamps rather than failing when either runs past the end.
    fn apply(self, input: &[u8]) -> &[u8] {
        let start = self.offset.min(input.len());
        let end = self.offset.saturating_add(self.length).min(input.len());
        &input[start..end]
    }
}

/// Every ROT13 shift whose result contains the crib.
pub(super) fn rot13_brute(
    input: &[u8],
    options: Rot13Options,
    sample: Sample,
    print_amount: bool,
    crib: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let window = sample.apply(input);
    let mut lines: Vec<String> = Vec::new();
    for amount in 1..26u8 {
        context.ensure_active()?;
        let rotated: Vec<u8> = window
            .iter()
            .map(|byte| rotate_rot13(*byte, options, amount))
            .collect();
        push_match(&mut lines, &rotated, amount, print_amount, crib);
    }
    context.ensure_active()?;
    Ok(lines.join("\n"))
}

/// Every ROT47 shift whose result contains the crib.
pub(super) fn rot47_brute(
    input: &[u8],
    sample: Sample,
    print_amount: bool,
    crib: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let window = sample.apply(input);
    let mut lines: Vec<String> = Vec::new();
    for amount in 1..94u8 {
        context.ensure_active()?;
        let rotated: Vec<u8> = window
            .iter()
            .map(|byte| rotate_rot47(*byte, amount))
            .collect();
        push_match(&mut lines, &rotated, amount, print_amount, crib);
    }
    context.ensure_active()?;
    Ok(lines.join("\n"))
}

/// Keeps a rotation whose decoded text contains the crib.
///
/// The comparison is case-insensitive on both sides and the crib is lowered
/// once by the caller, matching `indexOf` against a lowered haystack. An empty
/// crib is contained by everything, so every amount survives — which is what
/// makes the default arguments show the full table.
fn push_match(lines: &mut Vec<String>, rotated: &[u8], amount: u8, print_amount: bool, crib: &str) {
    let decoded = byte_array_to_utf8(rotated);
    if !decoded.to_lowercase().contains(crib) {
        return;
    }
    let escaped = escape_whitespace(&decoded);
    if print_amount {
        // `(" " + amount).slice(-2)` right-aligns in two columns.
        let mut line = String::from("Amount = ");
        if amount < 10 {
            line.push(' ');
        }
        line.push_str(&decimal(amount));
        line.push_str(": ");
        line.push_str(&escaped);
        lines.push(line);
    } else {
        lines.push(escaped);
    }
}

fn decimal(value: u8) -> String {
    if value == 0 {
        return String::from("0");
    }
    let mut digits = Vec::new();
    let mut remaining = value;
    while remaining > 0 {
        digits.push(b'0' + remaining % 10);
        remaining /= 10;
    }
    digits.reverse();
    digits.iter().map(|byte| char::from(*byte)).collect()
}

/// Moves control characters into the private use area so they stay visible.
///
/// The reference's range is `\x09-\x10`, which is tab through data-link
/// escape. That is very likely meant to be `\x09-\x0f` or similar, but the
/// range as written is what the output is measured against.
fn escape_whitespace(value: &str) -> String {
    value
        .chars()
        .map(|symbol| {
            if ('\u{09}'..='\u{10}').contains(&symbol) {
                char::from_u32(0xe000 + symbol as u32).unwrap_or(symbol)
            } else {
                symbol
            }
        })
        .collect()
}

const fn rotate_rot13(byte: u8, options: Rot13Options, amount: u8) -> u8 {
    if options.lower && byte >= b'a' && byte < b'a' + 26 {
        (byte - b'a' + amount) % 26 + b'a'
    } else if options.upper && byte >= b'A' && byte < b'A' + 26 {
        (byte - b'A' + amount) % 26 + b'A'
    } else if options.numbers && byte >= b'0' && byte < b'0' + 10 {
        (byte - b'0' + amount) % 10 + b'0'
    } else {
        byte
    }
}

const fn rotate_rot47(byte: u8, amount: u8) -> u8 {
    if byte >= 33 && byte <= 126 {
        (byte - 33 + amount) % 94 + 33
    } else {
        byte
    }
}
