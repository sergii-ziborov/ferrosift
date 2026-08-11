//! Deterministic compatibility findings.

use alloc::string::String;

/// Impact of a compatibility divergence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingSeverity {
    /// The source cannot be converted into executable portable IR.
    Error,
}

/// One source-ordered compatibility divergence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityFinding {
    /// Stable machine-readable code.
    pub code: &'static str,
    /// Compatibility impact.
    pub severity: FindingSeverity,
    /// Zero-based source step index.
    pub source_step: usize,
    /// Original operation name when one was present.
    pub original_operation: Option<String>,
    /// Human-readable explanation.
    pub explanation: String,
}

impl CompatibilityFinding {
    pub(crate) fn error(
        code: &'static str,
        source_step: usize,
        original_operation: Option<&str>,
        explanation: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: FindingSeverity::Error,
            source_step,
            original_operation: original_operation.map(String::from),
            explanation: explanation.into(),
        }
    }
}
