//! The output ceiling both codecs are bounded by.
//!
//! Shared rather than duplicated because the reason is shared: a decompressor
//! turns a small input into a large one, so the budget has to reach it *before*
//! it allocates rather than after. Two copies of that rule would eventually be
//! two rules.

use ferrosift_core::{OperationContext, OperationError};

/// The budget's output ceiling, as the decompressors want it.
///
/// A budget larger than the address space saturates rather than wrapping, which
/// is why [`ensure_fits`] still runs after every decompression: on a 32-bit
/// target a caller may legitimately set a ceiling this cannot express, and the
/// check afterwards is then the one that holds.
pub(super) fn output_limit(context: &OperationContext<'_>) -> usize {
    usize::try_from(context.budget().max_output_bytes).unwrap_or(usize::MAX)
}

/// Refuses an output the budget cannot hold.
pub(super) fn ensure_fits(
    output: &[u8],
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
