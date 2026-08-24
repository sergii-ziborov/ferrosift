//! Line-oriented text operations: tail, line numbering, and padding.

mod codec;
mod operation;

pub use operation::{AddLineNumbers, PadLines, RemoveLineNumbers, Tail};
