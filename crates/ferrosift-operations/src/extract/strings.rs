//! UNIX-like strings extraction for single-byte and UTF-16 encodings.

use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use super::common::{PresentFlags, ensure_output, finalize, format_results};
use crate::failure::failed;

const INVALID_ENCODING: &str = "extract.strings.invalid_encoding";
const INVALID_LENGTH: &str = "extract.strings.invalid_length";

#[derive(Clone, Copy)]
struct ClassFlags {
    bits: u8,
}

impl ClassFlags {
    const ALNUM_PUNCT: u8 = 0b01;
    const NULL_TERM: u8 = 0b10;

    fn from_match_type(match_type: &str) -> Self {
        let mut bits = 0;
        if match_type.contains("Alphanumeric + punctuation") {
            bits |= Self::ALNUM_PUNCT;
        }
        if match_type.contains("Null-terminated") {
            bits |= Self::NULL_TERM;
        }
        Self { bits }
    }

    fn alnum_punct(self) -> bool {
        self.bits & Self::ALNUM_PUNCT != 0
    }

    fn null_terminated(self) -> bool {
        self.bits & Self::NULL_TERM != 0
    }
}

pub(super) fn extract(
    input: &str,
    encoding: &str,
    min_len: i128,
    match_type: &str,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if min_len < 1 {
        return Err(failed(INVALID_LENGTH));
    }
    let min_len = usize::try_from(min_len).map_err(|_| failed(INVALID_LENGTH))?;
    let class = ClassFlags::from_match_type(match_type);
    let bytes = latin1_bytes(input);
    let mut results = Vec::new();
    match encoding {
        "Single byte" => results.extend(scan_single(&bytes, min_len, class)),
        "16-bit littleendian" => results.extend(scan_utf16(&bytes, min_len, true, class)),
        "16-bit bigendian" => results.extend(scan_utf16(&bytes, min_len, false, class)),
        "All" => {
            results.extend(scan_single(&bytes, min_len, class));
            results.extend(scan_utf16(&bytes, min_len, true, class));
            results.extend(scan_utf16(&bytes, min_len, false, class));
        }
        _ => return Err(failed(INVALID_ENCODING)),
    }
    let results = finalize(results, present.sort(), present.unique(), false, context)?;
    let output = format_results(&results, present.display_total());
    ensure_output(&output, context)?;
    Ok(output)
}

fn latin1_bytes(input: &str) -> Vec<u8> {
    input
        .encode_utf16()
        .map(|unit| u8::try_from(unit & 0xff).unwrap_or(0))
        .collect()
}

fn is_match_byte(byte: u8, class: ClassFlags) -> bool {
    if class.alnum_punct() {
        matches!(
            byte,
            b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'/'
                | b'\\'
                | b'-'
                | b':'
                | b'.'
                | b','
                | b'_'
                | b'$'
                | b'%'
                | b'\''
                | b'"'
                | b'('
                | b')'
                | b'<'
                | b'>'
                | b'='
                | b' '
                | b'!'
                | b'['
                | b']'
                | b'{'
                | b'}'
                | b'@'
        )
    } else {
        (0x20..=0x7e).contains(&byte)
    }
}

fn scan_single(bytes: &[u8], min_len: usize, class: ClassFlags) -> Vec<String> {
    let mut results = Vec::new();
    let mut current = Vec::new();
    for &byte in bytes {
        if is_match_byte(byte, class) {
            current.push(byte);
        } else {
            push_run(
                &mut results,
                &mut current,
                min_len,
                class.null_terminated(),
                byte == 0,
            );
        }
    }
    push_run(
        &mut results,
        &mut current,
        min_len,
        class.null_terminated(),
        false,
    );
    results
}

fn scan_utf16(bytes: &[u8], min_len: usize, little: bool, class: ClassFlags) -> Vec<String> {
    let mut results = Vec::new();
    let mut current = Vec::new();
    let mut index = 0_usize;
    while index + 1 < bytes.len() {
        let (lo, hi) = if little {
            (bytes[index], bytes[index + 1])
        } else {
            (bytes[index + 1], bytes[index])
        };
        if hi == 0 && is_match_byte(lo, class) {
            current.push(lo);
            index += 2;
        } else {
            let was_null = lo == 0 && hi == 0;
            push_run(
                &mut results,
                &mut current,
                min_len,
                class.null_terminated(),
                was_null,
            );
            index += 2;
        }
    }
    push_run(
        &mut results,
        &mut current,
        min_len,
        class.null_terminated(),
        false,
    );
    results
}

fn push_run(
    results: &mut Vec<String>,
    current: &mut Vec<u8>,
    min_len: usize,
    null_terminated: bool,
    ended_with_null: bool,
) {
    if current.len() >= min_len && (!null_terminated || ended_with_null) {
        if let Ok(text) = core::str::from_utf8(current) {
            results.push(String::from(text));
        } else {
            results.push(current.iter().map(|b| char::from(*b)).collect());
        }
    }
    current.clear();
}
