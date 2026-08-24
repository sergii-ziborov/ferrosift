//! Shared vocabulary for the suggestion search.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

pub(super) const MAX_DEPTH: i128 = 3;
pub(super) const MAX_RESULTS_CAP: i128 = 32;
pub(super) const PREVIEW_CHARS: usize = 64;

/// One candidate operation, described well enough to re-run it.
#[derive(Clone, Copy)]
pub(super) struct Step {
    pub id: &'static str,
    pub alias: &'static str,
    pub args_summary: &'static str,
    pub recipe_fragment: &'static str,
}

/// A scored candidate recipe and what it produced.
pub(super) struct Hit {
    pub score: u16,
    pub steps: Vec<Step>,
    pub reason: String,
    pub preview: String,
}

/// Search bounds carried through the recursion.
#[derive(Clone, Copy)]
pub(super) struct Options {
    pub depth: usize,
    pub intensive: bool,
}

pub(super) fn ensure_budget(
    len: usize,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if u64::try_from(len).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
