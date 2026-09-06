//! JWT Decode — payload only, no signature verification.

mod codec;
mod operation;

pub use operation::JwtDecode;
