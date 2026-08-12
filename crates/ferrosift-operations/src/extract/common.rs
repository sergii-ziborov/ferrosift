//! Shared extract result formatting (totals / sort / unique).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

/// Formats extract hits the way `CyberChef` does.
pub(super) fn format_results(results: &[String], display_total: bool) -> String {
    if display_total {
        let mut output = String::from("Total found: ");
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
    mut results: Vec<String>,
    sort: bool,
    unique: bool,
    sort_ip: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<String>, OperationError> {
    context.ensure_active()?;
    if sort {
        if sort_ip {
            results.sort_by_key(|value| ip_key(value));
        } else {
            results.sort_by_key(|value| value.to_ascii_lowercase());
        }
    }
    if unique {
        let mut seen = alloc::collections::BTreeSet::new();
        results.retain(|value| seen.insert(value.to_ascii_lowercase()));
    }
    context.ensure_active()?;
    Ok(results)
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
