//! Cooperative cancellation flags.

use ferrosift_core::{Cancellation, FlagCancellation};

#[test]
fn a_flag_starts_clear_and_reports_after_cancel() {
    let flag = FlagCancellation::new();
    assert!(!flag.is_cancelled());
    flag.cancel();
    assert!(flag.is_cancelled());
    flag.reset();
    assert!(!flag.is_cancelled());
}
