//! Caret/M decoding, case-insensitive regex folding, and power sets.

mod codec;
mod operation;

pub use operation::{CaretMDecode, FromCaseInsensitiveRegex, PowerSet};
