//! Binary-coded decimal: one nibble per decimal digit.
//!
//! The arithmetic is nothing. Everything here is packing, and four arguments
//! interact to decide it: which nibble stands for which digit, whether two
//! nibbles share a byte, whether a sign nibble is appended, and how the result
//! is written. The interactions are the subject — the sign nibble forces a
//! leading zero only when the digits are packed *and* there is an even number
//! of them, and an unpacked reading throws away every other nibble by a rule
//! that reads like a mistake and is not.
//!
//! Alone among the operations that take a number, this pair needs no
//! arbitrary-precision arithmetic: it never adds anything. It reads the
//! digits the dish already rendered and writes digits back, so it carries no
//! feature gate.

mod codec;
mod operation;

pub use operation::{FromBcd, ToBcd};
