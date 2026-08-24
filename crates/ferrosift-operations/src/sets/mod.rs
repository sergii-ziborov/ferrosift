//! Set operations and edit distances over delimited samples.

mod codec;
mod operation;

pub use operation::{HammingDistance, LevenshteinDistance, SetOperation};
