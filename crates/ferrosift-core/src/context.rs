//! Explicit ambient state passed to an operation.

use ferrosift_model::CapabilitySet;

use crate::{Cancellation, ExecutionBudget, OperationError};

/// Caller-controlled state available during one operation execution.
pub struct OperationContext<'a> {
    budget: ExecutionBudget,
    cancellation: &'a dyn Cancellation,
    capabilities: CapabilitySet,
}

impl<'a> OperationContext<'a> {
    /// Creates a context with explicit resource ceilings, cancellation, and grants.
    #[must_use]
    pub fn new(
        budget: ExecutionBudget,
        cancellation: &'a dyn Cancellation,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            budget,
            cancellation,
            capabilities,
        }
    }

    /// Returns the immutable resource ceilings for this execution.
    #[must_use]
    pub const fn budget(&self) -> &ExecutionBudget {
        &self.budget
    }

    /// Returns the host capabilities explicitly granted to this execution.
    #[must_use]
    pub const fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    /// Fails closed when the caller has requested cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::Cancelled`] when cancellation was requested.
    pub fn ensure_active(&self) -> Result<(), OperationError> {
        if self.cancellation.is_cancelled() {
            Err(OperationError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Refuses an allocation the operation is about to make.
    ///
    /// Asked *before* allocating, which is the whole point: an operation that
    /// has already taken the memory has already done the harm, and reporting
    /// it afterwards is a description rather than a limit. Only operations
    /// whose transient cost is set by an argument need to ask — for everything
    /// else it is bounded by the input, which the budget already sees.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::TransientLimitExceeded`] when `bytes` is above
    /// the budget's ceiling.
    pub fn ensure_transient(&self, bytes: u64) -> Result<(), OperationError> {
        if bytes > self.budget.max_transient_bytes {
            Err(OperationError::TransientLimitExceeded)
        } else {
            Ok(())
        }
    }

    /// Refuses work the operation is about to perform.
    ///
    /// The estimate is the operation's own and does not have to be accurate in
    /// seconds — it has to be *monotonic* in the arguments that drive the cost,
    /// so that asking for a thousand times more work is refused a thousand
    /// times sooner. See [`ExecutionBudget::max_work_units`].
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::WorkLimitExceeded`] when `units` is above the
    /// budget's ceiling.
    pub fn ensure_work(&self, units: u64) -> Result<(), OperationError> {
        if units > self.budget.max_work_units {
            Err(OperationError::WorkLimitExceeded)
        } else {
            Ok(())
        }
    }
}
