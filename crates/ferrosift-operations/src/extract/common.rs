//! Shared extract result formatting (totals / sort / unique).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

/// How extract results are ordered when sort is enabled.
#[derive(Clone, Copy)]
pub(super) enum SortKey {
    /// Case-insensitive ASCII order.
    AsciiIgnoreCase,
    /// Numeric IPv4 order, then remaining strings.
    Ipv4,
    /// `CyberChef` hexadecimalSort for MAC-like tokens.
    Hexadecimal,
}

/// How extract results are deduplicated when unique is enabled.
#[derive(Clone, Copy)]
pub(super) enum UniqueKey {
    /// Case-insensitive first-seen retention.
    IgnoreCase,
    /// Exact-string first-seen retention.
    Exact,
}

/// Formats extract hits the way `CyberChef` does (`Total found:`).
pub(super) fn format_results(results: &[String], display_total: bool) -> String {
    format_results_labeled(results, display_total, "Total found")
}

/// Formats extract hits with a custom total label (`Total found` / `Total Results`).
pub(super) fn format_results_labeled(
    results: &[String],
    display_total: bool,
    label: &str,
) -> String {
    if display_total {
        let mut output = String::from(label);
        output.push_str(": ");
        output.push_str(&results.len().to_string());
        output.push_str("\n\n");
        output.push_str(&results.join("\n"));
        output
    } else {
        results.join("\n")
    }
}

/// Applies optional sort and order-preserving unique.
pub(super) fn finalize(
    results: Vec<String>,
    sort: bool,
    unique: bool,
    sort_ip: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<String>, OperationError> {
    finalize_with(
        results,
        sort,
        unique,
        if sort_ip {
            SortKey::Ipv4
        } else {
            SortKey::AsciiIgnoreCase
        },
        UniqueKey::IgnoreCase,
        context,
    )
}

/// Applies optional sort / unique with explicit key modes.
pub(super) fn finalize_with(
    mut results: Vec<String>,
    sort: bool,
    unique: bool,
    sort_key: SortKey,
    unique_key: UniqueKey,
    context: &OperationContext<'_>,
) -> Result<Vec<String>, OperationError> {
    context.ensure_active()?;
    if sort {
        match sort_key {
            SortKey::AsciiIgnoreCase => results.sort_by_key(|value| value.to_ascii_lowercase()),
            SortKey::Ipv4 => results.sort_by_key(|value| ip_key(value)),
            SortKey::Hexadecimal => results.sort_by(|left, right| hexadecimal_cmp(left, right)),
        }
    }
    if unique {
        match unique_key {
            UniqueKey::IgnoreCase => {
                let mut seen = alloc::collections::BTreeSet::new();
                results.retain(|value| seen.insert(value.to_ascii_lowercase()));
            }
            UniqueKey::Exact => {
                let mut seen = alloc::collections::BTreeSet::new();
                results.retain(|value| seen.insert(value.clone()));
            }
        }
    }
    context.ensure_active()?;
    Ok(results)
}

fn hexadecimal_cmp(left: &str, right: &str) -> core::cmp::Ordering {
    let left_parts = hex_parts(left);
    let right_parts = hex_parts(right);
    let len = left_parts.len().min(right_parts.len());
    for index in 0..len {
        match (&left_parts[index], &right_parts[index]) {
            (HexPart::Number(a), HexPart::Number(b)) => {
                if a != b {
                    return a.cmp(b);
                }
            }
            (HexPart::Text(a), HexPart::Text(b)) => {
                let order = a.cmp(b);
                if order != core::cmp::Ordering::Equal {
                    return order;
                }
            }
            (HexPart::Text(_), HexPart::Number(_)) => return core::cmp::Ordering::Greater,
            (HexPart::Number(_), HexPart::Text(_)) => return core::cmp::Ordering::Less,
        }
    }
    left.cmp(right)
}

enum HexPart {
    Number(u128),
    Text(String),
}

fn hex_parts(value: &str) -> Vec<HexPart> {
    let mut parts = Vec::new();
    let mut index = 0;
    let bytes = value.as_bytes();
    while index < bytes.len() {
        let start = index;
        let is_hex = bytes[index].is_ascii_hexdigit();
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_hexdigit() == is_hex {
            index += 1;
        }
        let chunk = &value[start..index];
        if is_hex {
            match u128::from_str_radix(chunk, 16) {
                Ok(number) => parts.push(HexPart::Number(number)),
                Err(_) => parts.push(HexPart::Text(String::from(chunk))),
            }
        } else {
            parts.push(HexPart::Text(String::from(chunk)));
        }
    }
    parts
}

fn ip_key(value: &str) -> (u8, u32, String) {
    match ipv4_key(value) {
        Some(key) => (0, key, String::new()),
        None => (1, 0, String::from(value)),
    }
}

fn ipv4_key(value: &str) -> Option<u32> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut key = 0_u32;
    for part in parts {
        let byte = part.parse::<u32>().ok()?;
        if byte > 255 {
            return None;
        }
        key = (key << 8) | byte;
    }
    Some(key)
}

pub(super) fn ensure_output(
    text: &str,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if u64::try_from(text.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}

/// Packed extract presentation flags.
#[derive(Clone, Copy)]
pub(super) struct PresentFlags {
    bits: u8,
}

impl PresentFlags {
    pub(super) const DISPLAY_TOTAL: u8 = 0b001;
    pub(super) const SORT: u8 = 0b010;
    pub(super) const UNIQUE: u8 = 0b100;

    pub(super) const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub(super) fn display_total(self) -> bool {
        self.bits & Self::DISPLAY_TOTAL != 0
    }

    pub(super) fn sort(self) -> bool {
        self.bits & Self::SORT != 0
    }

    pub(super) fn unique(self) -> bool {
        self.bits & Self::UNIQUE != 0
    }
}
