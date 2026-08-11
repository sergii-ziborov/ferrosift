//! Explicit resource ceilings for operation execution.

/// Resource ceilings visible to an operation and enforced by an executor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionBudget {
    /// Maximum recipe steps that may be evaluated.
    pub max_steps: usize,
    /// Maximum accepted input size in bytes.
    pub max_input_bytes: u64,
    /// Maximum produced output size in bytes.
    pub max_output_bytes: u64,
    /// Maximum output-to-input expansion ratio.
    pub max_expansion_ratio: u32,
}
