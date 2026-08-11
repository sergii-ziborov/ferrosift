//! Stable identifiers for operation contracts.

use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::ModelError;

/// A versioned, cross-platform identifier for an operation contract.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct OperationId(String);

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
            Ok(Self(value))
        } else {
            Err(ModelError::InvalidOperationId)
        }
    }

    /// Borrows the canonical identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
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

fn is_valid_operation_id(value: &str) -> bool {
    let Some((path, major)) = value.rsplit_once('@') else {
        return false;
    };

    !path.contains('@')
        && path.split('.').all(is_valid_segment)
        && is_canonical_major_version(major)
}

fn is_valid_segment(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    bytes.next().is_some_and(is_lowercase_or_digit)
        && bytes.all(|byte| is_lowercase_or_digit(byte) || matches!(byte, b'_' | b'-'))
}

fn is_lowercase_or_digit(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

fn is_canonical_major_version(version: &str) -> bool {
    !version.is_empty()
        && version.bytes().all(|byte| byte.is_ascii_digit())
        && (version == "0" || !version.starts_with('0'))
}
