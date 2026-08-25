//! Case transforms: upper, lower, swapped, alternating, and every casing.

mod codec;
mod operation;

pub use operation::{AlternatingCaps, GetAllCasings, SwapCase, ToLowerCase, ToUpperCase};
