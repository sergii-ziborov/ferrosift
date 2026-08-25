//! IEEE-754 floating point.
//!
//! Two things here are not the obvious implementation.
//!
//! A not-a-number value is written as `7f800001`, not the `7fc00000` that
//! Rust's `as f32` produces. The reference's packer builds the bytes by hand:
//! exponent all ones, mantissa exactly one, sign clear because `NaN < 0` is
//! false. Both mean NaN and only one is the bytes the reference emits.
//!
//! Reading back goes through [`crate::jscompat::double`] rather than Rust's
//! own formatting, because JavaScript switches to exponential notation above
//! `1e21` and below `1e-6` and Rust does neither. `1e-9` is `1e-9` there and
//! `0.000000001` here, for the same double.

mod codec;
mod operation;

pub use operation::{FromFloat, ToFloat};
