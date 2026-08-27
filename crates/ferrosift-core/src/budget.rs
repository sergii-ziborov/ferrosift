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
    /// Maximum memory one operation may hold *while running*, beyond its
    /// answer.
    ///
    /// Every other ceiling here measures something that crosses a boundary:
    /// bytes in, bytes out, steps taken. This measures what an operation asks
    /// for in the middle, which no boundary sees. scrypt is the case that made
    /// it necessary — it takes its memory from an argument, `128 * r * N`, and
    /// returns thirty-two bytes.
    pub max_transient_bytes: u64,
    /// Maximum work one operation may perform, in abstract units.
    ///
    /// A unit is roughly one compression-function call: one hash block, one
    /// scrypt mixing round. The scale is chosen so estimates are comparable
    /// across operations rather than accurate in seconds — what matters is
    /// that a recipe cannot ask for a billion of them and be handed a
    /// sixteen-byte answer.
    ///
    /// This is also the only bound on how long an operation is *unresponsive*.
    /// Cancellation is cooperative and a library call cannot be interrupted
    /// from outside, so bounding the work declared before the call is what
    /// bounds the window in which nothing can stop it.
    pub max_work_units: u64,
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
            // Enough for scrypt at `N = 2^17, r = 8` (128 MiB), which is far
            // above any parameter a person picks, and short of the gigabytes
            // the next power of two would ask for.
            max_transient_bytes: 256 * 1024 * 1024,
            // Sixty-seven million: PBKDF2 at about thirty million iterations
            // for a single block, where the strongest published guidance asks
            // for six hundred thousand. Generous by two orders of magnitude
            // against real use, and two short of the four billion the argument
            // will otherwise accept.
            max_work_units: 1 << 26,
        }
    }

    /// The largest output an input-proportional step reading `input_size` bytes
    /// can produce and still be accepted.
    ///
    /// The executor applies this after the fact, which is the right place for
    /// almost everything: an operation that has produced its answer has already
    /// paid for it, and measuring is free. It is the wrong place when producing
    /// the answer is itself the expense — arbitrary-precision arithmetic can
    /// turn two short numbers into tens of millions of digits, and the executor
    /// then refuses what it cost seconds to build.
    ///
    /// So the rule is stated once and read from both ends. An operation that
    /// can predict its answer's size cheaply may compare against this first and
    /// refuse without building.
    ///
    /// One arm of the executor's check is missing here, deliberately: it also
    /// compares against the *recipe's* original input, which an operation
    /// cannot see. That only makes this ceiling equal or higher, so a value
    /// above it would have been refused either way.
    #[must_use]
    pub fn output_ceiling(&self, input_size: u64) -> u64 {
        // An empty input still gets a ratio's worth, or a generator called the
        // way generators are called would be refused everything.
        let proportional = input_size
            .max(1)
            .saturating_mul(u64::from(self.max_expansion_ratio));
        self.max_output_bytes.min(proportional)
    }
}
