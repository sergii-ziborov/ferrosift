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
    /// Maximum branches produced by a single Fork split.
    pub max_branches: usize,
    /// Maximum nested Fork/flow depth.
    pub max_flow_depth: usize,
    /// Maximum total operation invocations across the whole execution
    /// (including every branch body step).
    pub max_operation_invocations: u64,
    /// Maximum total bytes processed (branch inputs + operation inputs).
    pub max_total_bytes_processed: u64,
}

impl ExecutionBudget {
    /// Generous defaults suitable for local CLI / tests without flow storms.
    #[must_use]
    pub const fn generous() -> Self {
        Self {
            max_steps: 4_096,
            max_input_bytes: 16 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
            max_expansion_ratio: 64,
            max_branches: 1_048_576,
            max_flow_depth: 64,
            max_operation_invocations: 10_000_000,
            max_total_bytes_processed: 256 * 1024 * 1024,
        }
    }
}
