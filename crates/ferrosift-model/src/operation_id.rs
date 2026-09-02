//! Stable identifiers for operation contracts.

use alloc::{borrow::Cow, string::String};
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::ModelError;

/// A versioned, cross-platform identifier for an operation contract.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct OperationId(Cow<'static, str>);

impl OperationId {
    /// Creates an operation ID after validating its canonical grammar.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::InvalidOperationId`] when `value` is not composed of
    /// lowercase ASCII segments followed by an explicit canonical major version.
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if is_valid_operation_id(&value) {
            Ok(Self(Cow::Owned(value)))
        } else {
            Err(ModelError::InvalidOperationId)
        }
    }

    /// Creates an operation ID from a built-in static value.
    ///
    /// This constructor allows built-in operation catalogs to validate their
    /// identifiers during constant evaluation without a runtime unwrap.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not follow the canonical operation-ID grammar.
    #[must_use]
    pub const fn from_static(value: &'static str) -> Self {
        assert!(is_valid_operation_id(value), "invalid static operation ID");
        Self(Cow::Borrowed(value))
    }

    /// Borrows the canonical identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The namespace this operation shares with its siblings.
    ///
    /// Everything before the last segment and the version, so
    /// `encoding.base64.decode@1` and `encoding.base64.encode@1` are both
    /// `encoding.base64`, and `hash.sha2@1` is `hash`. A *cluster* is the set
    /// of operations that answer to one namespace.
    ///
    /// This is a reading of the id rather than a new field, because the
    /// grouping is already there: whoever named the operations put the couples
    /// and the families next to each other, and the sort order of a catalog
    /// listing shows it. Naming the reading is what lets it be checked —
    /// `tests/clusters.rs` requires an operation's declared inverse to live in
    /// its own cluster, which turns the convention into something a new
    /// operation cannot quietly break.
    ///
    /// The grouping is coarse at the top level on purpose. An operation named
    /// directly under its family, like `logic.xor@1`, clusters with every other
    /// `logic.*`, and that is the honest answer: the id says they are siblings
    /// and nothing finer was recorded.
    #[must_use]
    pub fn cluster(&self) -> &str {
        let bare = self
            .as_str()
            .split_once('@')
            .map_or_else(|| self.as_str(), |(name, _)| name);
        bare.rsplit_once('.')
            .map_or(bare, |(namespace, _)| namespace)
    }
}

impl AsRef<str> for OperationId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for OperationId {
    type Error = ModelError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for OperationId {
    type Error = ModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for OperationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

const fn is_valid_operation_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut separator = usize::MAX;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'@' {
            if separator != usize::MAX {
                return false;
            }
            separator = index;
        }
        index += 1;
    }
    if separator == usize::MAX || separator == 0 || separator + 1 >= bytes.len() {
        return false;
    }

    let mut segment_start = true;
    index = 0;
    while index < separator {
        let byte = bytes[index];
        if byte == b'.' {
            if segment_start {
                return false;
            }
            segment_start = true;
        } else if segment_start {
            if !is_lowercase_or_digit(byte) {
                return false;
            }
            segment_start = false;
        } else if !is_lowercase_or_digit(byte) && byte != b'_' && byte != b'-' {
            return false;
        }
        index += 1;
    }
    if segment_start {
        return false;
    }

    index = separator + 1;
    if bytes[index] == b'0' && index + 1 < bytes.len() {
        return false;
    }
    while index < bytes.len() {
        if !is_digit(bytes[index]) {
            return false;
        }
        index += 1;
    }
    true
}

const fn is_lowercase_or_digit(byte: u8) -> bool {
    (byte >= b'a' && byte <= b'z') || is_digit(byte)
}

const fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}
