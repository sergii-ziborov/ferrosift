//! Operations that remove or reorder parts of the input.

mod codec;
mod select;
mod whitespace;

pub use select::{DropNthBytes, Reverse, TakeNthBytes};
pub use whitespace::{RemoveNullBytes, RemoveWhitespace};
