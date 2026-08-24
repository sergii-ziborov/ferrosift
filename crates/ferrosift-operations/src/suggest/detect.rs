//! Shape detectors and fixed argument sets used by the probes.

use alloc::string::String;

use ferrosift_model::{ArgumentValue, Arguments};

pub(super) fn looks_like_hex(text: &str) -> bool {
    let mut digits = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_hexdigit() {
            digits += 1;
        } else if ch.is_ascii_whitespace() || matches!(ch, ':' | ',' | '-' | 'x' | 'X' | '0') {
        } else {
            other += 1;
        }
    }
    digits >= 8 && digits.is_multiple_of(2) && other * 4 <= digits
}

pub(super) fn looks_like_base64(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.len() < 8 {
        return false;
    }
    let body = compact.trim_end_matches('=');
    if !body
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
    {
        return false;
    }
    compact.len().is_multiple_of(4)
}

pub(super) fn looks_like_base32(text: &str) -> bool {
    let compact: String = text
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 8 {
        return false;
    }
    let body = compact.trim_end_matches('=');
    body.chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7')) && body.len() * 5 >= 40
}

pub(super) fn looks_like_url_encoded(text: &str) -> bool {
    text.contains('%')
        && text
            .as_bytes()
            .windows(3)
            .any(|w| w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit())
}

pub(super) fn looks_like_html_entities(text: &str) -> bool {
    text.contains("&lt;")
        || text.contains("&gt;")
        || text.contains("&amp;")
        || text.contains("&#")
        || text.contains("&quot;")
}

pub(super) fn looks_defanged(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("hxxp") || lower.contains("[.]") || lower.contains("[://]")
}

pub(super) fn looks_like_zlib(bytes: &[u8]) -> bool {
    if bytes.len() < 2 {
        return false;
    }
    let cmf = bytes[0];
    let flg = bytes[1];
    cmf & 0x0f == 8 && (u16::from(cmf) * 256 + u16::from(flg)).is_multiple_of(31)
}

pub(super) fn looks_mostly_alpha(bytes: &[u8]) -> bool {
    if bytes.len() < 8 {
        return false;
    }
    let alpha = bytes
        .iter()
        .filter(|b| b.is_ascii_alphabetic() || b.is_ascii_whitespace())
        .count();
    alpha * 100 / bytes.len() >= 80
}

pub(super) fn zlib_args() -> Arguments {
    Arguments::from([
        ("start_index".into(), ArgumentValue::Integer(0)),
        (
            "initial_output_buffer_size".into(),
            ArgumentValue::Integer(0),
        ),
        (
            "buffer_expansion_type".into(),
            ArgumentValue::Text("Adaptive".into()),
        ),
        (
            "resize_buffer_after_decompression".into(),
            ArgumentValue::Boolean(false),
        ),
        ("verify_result".into(), ArgumentValue::Boolean(false)),
    ])
}
