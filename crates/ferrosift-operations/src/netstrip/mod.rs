//! Dropping a protocol header to get at the payload underneath.

mod codec;
mod operation;

pub use operation::StripHeader;
