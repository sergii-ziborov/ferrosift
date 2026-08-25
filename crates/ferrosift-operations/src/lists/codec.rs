//! List reshaping over a delimited string.

use alloc::string::String;
use alloc::vec::Vec;

use crate::jscompat::object::KeySet;

/// Splits the way JavaScript's `String.prototype.split` does.
///
/// An empty delimiter separates characters rather than returning the whole
/// string, which is why this is not `str::split`. JavaScript separates *code
/// units* there, so an astral character comes apart into two lone surrogates;
/// Rust strings cannot hold those, so this separates characters instead. The
/// two agree on everything in the Basic Multilingual Plane and part company on
/// emoji, which `docs/compatibility/cyberchef-v11.3.0.md` records.
#[must_use]
pub fn split<'a>(input: &'a str, delimiter: &str) -> Vec<&'a str> {
    if delimiter.is_empty() {
        if input.is_empty() {
            // `"".split("")` is the empty list, not a list holding one empty
            // string — the one place the two disagree about emptiness.
            return Vec::new();
        }
        let mut parts = Vec::new();
        let mut start = 0;
        for (at, _) in input.char_indices().skip(1) {
            parts.push(&input[start..at]);
            start = at;
        }
        parts.push(&input[start..]);
        return parts;
    }
    input.split(delimiter).collect()
}

/// Removes repeated entries, keeping the first of each.
#[must_use]
pub fn unique(input: &str, delimiter: &str) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for part in split(input, delimiter) {
        if !seen.contains(&part) {
            seen.push(part);
        }
    }
    seen.join(delimiter)
}

/// Counts each distinct entry, in JavaScript object-key order.
///
/// Not first-seen order. The reference accumulates into a plain object and
/// then reads `Object.keys`, which hands back integer-like keys first in
/// ascending numeric order and only then the rest in insertion order. So
/// counting `["b", "2", "a", "1"]` lists `2` and `1` before `b` and `a`, and
/// reordering them as `1, 2` while it is at it.
#[must_use]
pub fn unique_with_counts(input: &str, delimiter: &str) -> String {
    let parts = split(input, delimiter);
    let mut keys = KeySet::new();
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for part in parts {
        keys.insert(part);
        if let Some(entry) = counts.iter_mut().find(|(name, _)| *name == part) {
            entry.1 += 1;
        } else {
            counts.push((part, 1));
        }
    }

    let ordered: Vec<String> = keys
        .keys()
        .into_iter()
        .map(|key| {
            let count = counts
                .iter()
                .find(|(name, _)| *name == key)
                .map_or(0, |(_, count)| *count);
            alloc::format!("{count} {key}")
        })
        .collect();
    ordered.join(delimiter)
}

/// Replaces one delimiter with another.
#[must_use]
pub fn respan(input: &str, from: &str, to: &str) -> String {
    split(input, from).join(to)
}
