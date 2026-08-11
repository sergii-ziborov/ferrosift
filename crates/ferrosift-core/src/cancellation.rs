//! Host-independent cooperative cancellation.

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
