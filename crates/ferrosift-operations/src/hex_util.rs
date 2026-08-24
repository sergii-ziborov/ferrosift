//! Shared lowercase hex rendering for digests and dumps.

use alloc::{string::String, vec::Vec};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Encodes bytes as lower-case hex without separators.
pub(crate) fn to_hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

/// The reference library's `fromHex(data, "Auto", 2)`.
///
/// This is the permissive helper other operations call, not the strict `From
/// Hex` operation: it splits on `/[^a-f\d]|0x/gi` and reads each run of hex
/// digits two at a time, so a trailing odd digit becomes its own byte rather
/// than an error. The alternation order matters — at `0x41` the `0x` branch
/// wins over matching `x` alone, which is the difference between one byte and
/// two.
pub(crate) fn from_hex_auto(input: &str) -> Vec<u8> {
    let characters: Vec<char> = input.chars().collect();
    let mut output = Vec::new();
    let mut digits: Vec<char> = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let current = characters[index];
        if !current.is_ascii_hexdigit() {
            flush(&mut digits, &mut output);
            index += 1;
            continue;
        }
        if current == '0' && matches!(characters.get(index + 1), Some('x' | 'X')) {
            flush(&mut digits, &mut output);
            index += 2;
            continue;
        }
        digits.push(current);
        index += 1;
    }
    flush(&mut digits, &mut output);
    output
}

/// Reads one accumulated run of hex digits, two at a time.
fn flush(digits: &mut Vec<char>, output: &mut Vec<u8>) {
    for pair in digits.chunks(2) {
        let mut value = 0u8;
        for digit in pair {
            let nibble = digit.to_digit(16).unwrap_or(0);
            value = (value << 4) | u8::try_from(nibble).unwrap_or(0);
        }
        output.push(value);
    }
    digits.clear();
}
