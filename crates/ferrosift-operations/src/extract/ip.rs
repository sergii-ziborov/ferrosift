//! IPv4 / IPv6 extraction without lookbehind (portable regex limits).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use super::common::{PresentFlags, ensure_output, finalize, format_results};

/// Packed IPv4/IPv6 selection flags.
#[derive(Clone, Copy)]
pub(super) struct IpFlags {
    bits: u8,
}

impl IpFlags {
    pub(super) const IPV4: u8 = 0b001;
    pub(super) const IPV6: u8 = 0b010;
    pub(super) const REMOVE_LOCAL: u8 = 0b100;

    pub(super) const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    fn ipv4(self) -> bool {
        self.bits & Self::IPV4 != 0
    }

    fn ipv6(self) -> bool {
        self.bits & Self::IPV6 != 0
    }

    fn remove_local(self) -> bool {
        self.bits & Self::REMOVE_LOCAL != 0
    }
}

pub(super) fn extract(
    input: &str,
    ip: IpFlags,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut results = Vec::new();
    if ip.ipv4() {
        results.extend(find_ipv4(input));
    }
    if ip.ipv6() {
        results.extend(find_ipv6(input));
    }
    if ip.remove_local() {
        results.retain(|value| !is_local_ipv4(value));
    }
    let results = finalize(results, present.sort(), present.unique(), true, context)?;
    let output = format_results(&results, present.display_total());
    ensure_output(&output, context)?;
    Ok(output)
}

fn find_ipv4(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut results = Vec::new();
    let mut index = 0_usize;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        if index > 0 && bytes[index - 1].is_ascii_digit() {
            index += 1;
            continue;
        }
        if let Some((end, value)) = match_ipv4(bytes, index) {
            if end < bytes.len() && bytes[end].is_ascii_digit() {
                index += 1;
                continue;
            }
            results.push(value);
            index = end;
        } else {
            index += 1;
        }
    }
    results
}

fn match_ipv4(bytes: &[u8], start: usize) -> Option<(usize, String)> {
    if let Some(end) = match_ipv4_form(bytes, start, false) {
        return Some((
            end,
            core::str::from_utf8(&bytes[start..end]).ok()?.to_string(),
        ));
    }
    if let Some(end) = match_ipv4_form(bytes, start, true) {
        return Some((
            end,
            core::str::from_utf8(&bytes[start..end]).ok()?.to_string(),
        ));
    }
    None
}

fn match_ipv4_form(bytes: &[u8], mut index: usize, octal: bool) -> Option<usize> {
    for group in 0..4 {
        if group > 0 {
            if index >= bytes.len() || bytes[index] != b'.' {
                return None;
            }
            index += 1;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return None;
        }
        let token = core::str::from_utf8(&bytes[start..index]).ok()?;
        if octal {
            if !is_octal_byte(token) {
                return None;
            }
        } else if !is_decimal_byte(token) {
            return None;
        }
    }
    Some(index)
}

fn is_decimal_byte(token: &str) -> bool {
    if token.len() > 3 {
        return false;
    }
    token.parse::<u16>().is_ok_and(|value| value <= 255)
}

fn is_octal_byte(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() || bytes[0] != b'0' {
        return false;
    }
    if bytes.len() == 1 {
        return true;
    }
    let rest = &bytes[1..];
    rest.len() <= 3 && rest.iter().all(|b| (b'0'..=b'7').contains(b))
}

fn find_ipv6(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut results = Vec::new();
    let mut index = 0_usize;
    while index < bytes.len() {
        if is_ipv6_start(bytes, index)
            && let Some((end, value)) = match_ipv6(bytes, index)
        {
            results.push(value);
            index = end;
            continue;
        }
        index += 1;
    }
    results
}

fn is_ipv6_start(bytes: &[u8], index: usize) -> bool {
    matches!(
        bytes.get(index),
        Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':')
    )
}

fn match_ipv6(bytes: &[u8], start: usize) -> Option<(usize, String)> {
    let mut index = start;
    let mut double = false;
    let mut hex_run = 0_usize;
    let mut groups = 0_usize;
    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {
                hex_run += 1;
                if hex_run > 4 {
                    break;
                }
                index += 1;
            }
            b':' => {
                if hex_run > 0 {
                    groups += 1;
                    hex_run = 0;
                }
                if index + 1 < bytes.len() && bytes[index + 1] == b':' {
                    if double {
                        break;
                    }
                    double = true;
                    index += 2;
                    continue;
                }
                index += 1;
            }
            _ => break,
        }
    }
    if hex_run > 0 {
        groups += 1;
    }
    if groups == 0 && !double {
        return None;
    }
    if !double && groups != 8 {
        return None;
    }
    if double && groups > 7 {
        return None;
    }
    if index - start < 2 {
        return None;
    }
    let text = core::str::from_utf8(&bytes[start..index]).ok()?.to_string();
    if !text.contains(':') {
        return None;
    }
    Some((index, text))
}

fn is_local_ipv4(value: &str) -> bool {
    value.starts_with("10.")
        || value.starts_with("192.168.")
        || value.starts_with("127.")
        || value
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|octet| octet.parse::<u8>().ok())
            .is_some_and(|second| (16..=31).contains(&second))
}
