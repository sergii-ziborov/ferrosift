//! SNORT hex content: pipe-delimited hex runs inside otherwise plain text.

mod codec;
mod operation;

pub use operation::{FromHexContent, ToHexContent};
