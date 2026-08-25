//! Two hashes that predate the ones people reach for: SHA-0 and MurmurHash3.

mod codec;
mod operation;

pub use operation::{MurmurHash3, Sha0};
