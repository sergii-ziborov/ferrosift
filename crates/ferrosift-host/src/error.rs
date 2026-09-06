//! Stable hosted-layer failures.

use std::fmt;

/// Result alias for the hosted layer.
pub type HostResult<T> = Result<T, HostError>;

/// Machine-readable hosted failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostError {
    code: String,
    detail: String,
}

impl HostError {
    /// Creates a failure with a stable code and human detail.
    #[must_use]
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    /// Stable machine-readable code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Human-readable detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ferrosift-host: {}: {}", self.code, self.detail)
    }
}

impl std::error::Error for HostError {}
