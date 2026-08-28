//! Arbitrary-precision integer arithmetic.
//!
//! The reference reaches for JavaScript's `BigInt` here, and a fixed-width
//! port would be a different operation: cryptographic moduli routinely exceed
//! sixty-four bits, which is most of the reason Modular Inverse exists at all.
//! `num-bigint` supplies the arithmetic, with default features off so the pack
//! still reaches both bare-metal targets.
//!
//! Everything in this module is exact, which is what separates it from its
//! neighbour. The reference's *decimal* operations — Sum, Divide, Mean and the
//! rest — live in [`crate::arith`], because they need `bignumber.js`'s
//! rounding mode, precision, and exponential-notation thresholds reproduced as
//! well as its arithmetic. Those rules are in [`crate::jscompat::bignumber`],
//! pinned against the library itself.

mod codec;
mod operation;

pub use operation::{ExtendedGcd, ModularExponentiation, ModularInverse};
