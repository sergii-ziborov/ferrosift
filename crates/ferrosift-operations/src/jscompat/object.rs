//! A key set with JavaScript plain-object semantics.
//!
//! `Set Union` builds its result in an object literal and reads it back with
//! `Object.keys`, and three consequences of that are observable in the output.
//! Anything named after an inherited `Object.prototype` member is skipped,
//! because the presence test finds the inherited function and believes the key
//! is already there. `__proto__` is skipped too, but for a different reason:
//! assigning to it changes the prototype rather than creating an own property.
//! And the surviving keys are not in insertion order — array-index-like names
//! come first, in ascending numeric order.
//!
//! Using a plain ordered set here would be tidier and would disagree.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// Names reachable through `Object.prototype`, whose values are truthy.
///
/// A truthy lookup makes the reference's `if (!hash[item])` test fail, so none
/// of these is ever added.
const PROTOTYPE_MEMBERS: [&str; 12] = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "__proto__",
];

/// Accumulates keys the way an object literal used as a set does.
pub(crate) struct KeySet {
    keys: Vec<String>,
}

impl KeySet {
    pub(crate) const fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Adds a key unless the object already appears to have it.
    pub(crate) fn insert(&mut self, key: &str) {
        if PROTOTYPE_MEMBERS.contains(&key) {
            return;
        }
        if self.keys.iter().any(|existing| existing == key) {
            return;
        }
        self.keys.push(key.to_string());
    }

    /// Returns the keys in `Object.keys` order.
    pub(crate) fn keys(self) -> Vec<String> {
        let (mut indices, names): (Vec<String>, Vec<String>) =
            self.keys.into_iter().partition(|key| is_array_index(key));
        indices.sort_by_key(|key| key.parse::<u32>().unwrap_or(0));
        indices.into_iter().chain(names).collect()
    }
}

/// Whether a property name is an array index, which `Object.keys` lists first.
///
/// The name must be the canonical decimal form of a value below 2^32 - 1, so
/// `"07"` and `"1.0"` are ordinary names while `"7"` is an index.
fn is_array_index(key: &str) -> bool {
    if key.is_empty() || (key.len() > 1 && key.starts_with('0')) {
        return false;
    }
    key.parse::<u32>().is_ok_and(|value| value != u32::MAX)
}
