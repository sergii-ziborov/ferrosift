//! Host-independent cooperative cancellation.

use core::sync::atomic::{AtomicBool, Ordering};

/// A cooperative cancellation signal supplied by the caller.
pub trait Cancellation {
    /// Returns whether execution should stop as soon as possible.
    fn is_cancelled(&self) -> bool;
}

/// A cancellation signal that never requests cancellation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NeverCancelled;

impl Cancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// A shared cancellation flag for hosted runtimes and interactive tools.
///
/// The host sets the flag; the executor polls it between steps. This is a
/// cooperative signal, not a hard interrupt of a single CPU-bound operation.
#[derive(Debug, Default)]
pub struct FlagCancellation {
    cancelled: AtomicBool,
}

impl FlagCancellation {
    /// Creates a flag that has not yet requested cancellation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    /// Requests that execution stop as soon as the executor next checks.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Clears a previous cancellation request.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Release);
    }
}

impl Cancellation for FlagCancellation {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
