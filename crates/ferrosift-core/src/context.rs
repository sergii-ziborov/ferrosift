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
}
