//! Additional digests: MD2, MD4, RIPEMD, SM3, and Whirlpool.
//!
//! These sit in the `hash` pack beside MD5 and the SHA family. The reference
//! parameterises several of them by round count so that reduced-round
//! variants can be studied; those are research constructions rather than the
//! published functions, so a non-standard count is refused here.

mod codec;
mod operation;

pub use operation::{Blake2, Blake3, FixedDigest, Ripemd, Streebog};
