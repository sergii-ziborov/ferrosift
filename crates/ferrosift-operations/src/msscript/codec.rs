//! Microsoft's encoded-script format, as used by `.vbe` and `.jse` files.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

/// Decodes the payload between the format's opening and closing markers.
///
/// Input that does not carry the markers yields the empty string rather than a
/// failure -- the reference tests for the pattern and returns `""` when it is
/// absent, so "not an encoded script" is not an error here.
pub(super) fn decode(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let Some(payload) = extract(input) else {
        return Ok(String::new());
    };

    // Five escapes are undone before the substitution runs, because the
    // characters they stand for are exactly the ones the substitution skips.
    let mut data: Vec<char> = Vec::with_capacity(payload.chars().count());
    let mut characters = payload.chars().peekable();
    while let Some(value) = characters.next() {
        if value != '@' {
            data.push(value);
            continue;
        }
        match characters.peek() {
            Some('&') => {
                characters.next();
                data.push('\n');
            }
            Some('#') => {
                characters.next();
                data.push('\r');
            }
            Some('*') => {
                characters.next();
                data.push('>');
            }
            Some('!') => {
                characters.next();
                data.push('<');
            }
            Some('$') => {
                characters.next();
                data.push('@');
            }
            _ => data.push('@'),
        }
    }

    let mut output = String::with_capacity(data.len());
    // The cycle counter advances only on characters below 128, so a
    // multi-byte character passes through without shifting the substitution
    // for everything after it.
    let mut position: usize = 0;
    let mut started = false;
    for value in data {
        let code = u32::from(value);
        if code < 128 {
            if started {
                position += 1;
            }
            started = true;
        }
        let substitutable =
            (code == 9 || (32..128).contains(&code)) && code != 60 && code != 62 && code != 64;
        if substitutable && started {
            let row = usize::try_from(code).unwrap_or(0);
            let column = COMBINATION[position % 64];
            output.push(char::from(DECODE[row][column]));
        } else {
            output.push(value);
        }
    }
    Ok(output)
}

/// Finds the payload between `#@~^......==` and `......==^#~@`.
///
/// The reference uses one greedy regular expression, so with more than one
/// encoded block in the input the match spans from the first opening marker to
/// the last closing one and the text between blocks is decoded as payload.
fn extract(input: &str) -> Option<&str> {
    let characters: Vec<char> = input.chars().collect();
    let open = find(&characters, &['#', '@', '~', '^'])?;
    // Six characters of header, then `==`.
    let start = open + 4 + 6 + 2;
    if start > characters.len() {
        return None;
    }
    let close = rfind(&characters, &['=', '=', '^', '#', '~', '@'])?;
    // Six characters of trailer precede the closing `==^#~@`.
    let end = close.checked_sub(6)?;
    if end < start {
        return None;
    }
    let byte_start = characters[..start].iter().map(|c| c.len_utf8()).sum();
    let byte_end = characters[..end].iter().map(|c| c.len_utf8()).sum();
    Some(&input[byte_start..byte_end])
}

/// First position of a character sequence.
fn find(haystack: &[char], needle: &[char]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Last position of a character sequence.
fn rfind(haystack: &[char], needle: &[char]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}
/// Three substitutions per byte; which one applies comes from the position.
///
/// Extracted from the reference table rather than retyped. An entry the
/// decoder never reaches is three zero bytes, which keeps the table square and
/// indexable by the byte itself.
const DECODE: [[u8; 3]; 128] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],
    [0x57, 0x6e, 0x7b],
    [0x4a, 0x4c, 0x41],
    [0x0b, 0x0b, 0x0b],
    [0x0c, 0x0c, 0x0c],
    [0x4a, 0x4c, 0x41],
    [0x0e, 0x0e, 0x0e],
    [0x0f, 0x0f, 0x0f],
    [0x10, 0x10, 0x10],
    [0x11, 0x11, 0x11],
    [0x12, 0x12, 0x12],
    [0x13, 0x13, 0x13],
    [0x14, 0x14, 0x14],
    [0x15, 0x15, 0x15],
    [0x16, 0x16, 0x16],
    [0x17, 0x17, 0x17],
    [0x18, 0x18, 0x18],
    [0x19, 0x19, 0x19],
    [0x1a, 0x1a, 0x1a],
    [0x1b, 0x1b, 0x1b],
    [0x1c, 0x1c, 0x1c],
    [0x1d, 0x1d, 0x1d],
    [0x1e, 0x1e, 0x1e],
    [0x1f, 0x1f, 0x1f],
    [0x2e, 0x2d, 0x32],
    [0x47, 0x75, 0x30],
    [0x7a, 0x52, 0x21],
    [0x56, 0x60, 0x29],
    [0x42, 0x71, 0x5b],
    [0x6a, 0x5e, 0x38],
    [0x2f, 0x49, 0x33],
    [0x26, 0x5c, 0x3d],
    [0x49, 0x62, 0x58],
    [0x41, 0x7d, 0x3a],
    [0x34, 0x29, 0x35],
    [0x32, 0x36, 0x65],
    [0x5b, 0x20, 0x39],
    [0x76, 0x7c, 0x5c],
    [0x72, 0x7a, 0x56],
    [0x43, 0x7f, 0x73],
    [0x38, 0x6b, 0x66],
    [0x39, 0x63, 0x4e],
    [0x70, 0x33, 0x45],
    [0x45, 0x2b, 0x6b],
    [0x68, 0x68, 0x62],
    [0x71, 0x51, 0x59],
    [0x4f, 0x66, 0x78],
    [0x09, 0x76, 0x5e],
    [0x62, 0x31, 0x7d],
    [0x44, 0x64, 0x4a],
    [0x23, 0x54, 0x6d],
    [0x75, 0x43, 0x71],
    [0x4a, 0x4c, 0x41],
    [0x7e, 0x3a, 0x60],
    [0x4a, 0x4c, 0x41],
    [0x5e, 0x7e, 0x53],
    [0x40, 0x4c, 0x40],
    [0x77, 0x45, 0x42],
    [0x4a, 0x2c, 0x27],
    [0x61, 0x2a, 0x48],
    [0x5d, 0x74, 0x72],
    [0x22, 0x27, 0x75],
    [0x4b, 0x37, 0x31],
    [0x6f, 0x44, 0x37],
    [0x4e, 0x79, 0x4d],
    [0x3b, 0x59, 0x52],
    [0x4c, 0x2f, 0x22],
    [0x50, 0x6f, 0x54],
    [0x67, 0x26, 0x6a],
    [0x2a, 0x72, 0x47],
    [0x7d, 0x6a, 0x64],
    [0x74, 0x39, 0x2d],
    [0x54, 0x7b, 0x20],
    [0x2b, 0x3f, 0x7f],
    [0x2d, 0x38, 0x2e],
    [0x2c, 0x77, 0x4c],
    [0x30, 0x67, 0x5d],
    [0x6e, 0x53, 0x7e],
    [0x6b, 0x47, 0x6c],
    [0x66, 0x34, 0x6f],
    [0x35, 0x78, 0x79],
    [0x25, 0x5d, 0x74],
    [0x21, 0x30, 0x43],
    [0x64, 0x23, 0x26],
    [0x4d, 0x5a, 0x76],
    [0x52, 0x5b, 0x25],
    [0x63, 0x6c, 0x24],
    [0x3f, 0x48, 0x2b],
    [0x7b, 0x55, 0x28],
    [0x78, 0x70, 0x23],
    [0x29, 0x69, 0x41],
    [0x28, 0x2e, 0x34],
    [0x73, 0x4c, 0x09],
    [0x59, 0x21, 0x2a],
    [0x33, 0x24, 0x44],
    [0x7f, 0x4e, 0x3f],
    [0x6d, 0x50, 0x77],
    [0x55, 0x09, 0x3b],
    [0x53, 0x56, 0x55],
    [0x7c, 0x73, 0x69],
    [0x3a, 0x35, 0x61],
    [0x5f, 0x61, 0x63],
    [0x65, 0x4b, 0x50],
    [0x46, 0x58, 0x67],
    [0x58, 0x3b, 0x51],
    [0x31, 0x57, 0x49],
    [0x69, 0x22, 0x4f],
    [0x6c, 0x6d, 0x46],
    [0x5a, 0x4d, 0x68],
    [0x48, 0x25, 0x7c],
    [0x27, 0x28, 0x36],
    [0x5c, 0x46, 0x70],
    [0x3d, 0x4a, 0x6e],
    [0x24, 0x32, 0x7a],
    [0x79, 0x41, 0x2f],
    [0x37, 0x3d, 0x5f],
    [0x60, 0x5f, 0x4b],
    [0x51, 0x4f, 0x5a],
    [0x20, 0x42, 0x2c],
    [0x36, 0x65, 0x57],
];

/// Which of the three substitutions each position in the 64-long cycle uses.
const COMBINATION: [usize; 64] = [
    0, 1, 2, 0, 1, 2, 1, 2, 2, 1, 2, 1, 0, 2, 1, 2, 0, 2, 1, 2, 0, 0, 1, 2, 2, 1, 0, 2, 1, 2, 2, 1,
    0, 0, 2, 1, 2, 1, 2, 0, 2, 0, 0, 1, 2, 0, 2, 1, 0, 2, 1, 2, 0, 0, 1, 2, 2, 0, 0, 1, 2, 0, 2, 1,
];
