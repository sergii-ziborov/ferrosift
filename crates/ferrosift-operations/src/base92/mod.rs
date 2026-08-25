//! Base92: thirteen bits per pair of symbols from a 91-letter alphabet.

mod codec;
mod operation;

pub use operation::{FromBase92, ToBase92};
