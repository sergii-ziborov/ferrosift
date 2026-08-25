//! Base62, a numeric encoding rather than a block encoding.
//!
//! Base64 and Base32 chop the input into fixed groups of bits; Base62 does not,
//! because 62 is not a power of two. The whole input is one integer, converted
//! by repeated division, which is why this needs arbitrary precision and why it
//! sits with [`crate::bigint`] rather than with the block codecs.
//!
//! Two consequences follow from that and are easy to trip over. Leading zero
//! bytes carry no value and do not survive the round trip, unlike Base58 next
//! door, which special-cases them. And the cost is quadratic in the input
//! length, so this is a notation for identifiers and short tokens, not a
//! transport encoding for files.

mod codec;
mod operation;

pub use operation::{FromBase62, ToBase62};
