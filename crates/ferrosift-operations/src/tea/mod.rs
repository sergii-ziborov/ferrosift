//! TEA and XTEA, the two block ciphers XXTEA came from.
//!
//! Sixty-four bit blocks and a hundred and twenty-eight bit key, wrapped in the
//! same five block modes AES offers here. Both are published with test vectors,
//! so the cipher is portable by construction; what is not portable by
//! construction is everything the reference wraps around it, which is why the
//! codec transcribes the modes and the padding rather than reaching for a
//! general block-mode layer.
//!
//! Neither pulls a dependency, so neither sits behind the `crypto` pack — the
//! same reason `XXTEA` beside them does not.

mod codec;
mod operation;

pub use operation::Tea;
