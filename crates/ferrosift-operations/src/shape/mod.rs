//! Reshaping text: ANSI stripping, HTTP framing, wrapping, and ranges.

mod codec;
mod operation;

pub use operation::{
    DechunkHttpResponse, ExpandAlphabetRange, RemoveAnsiEscapeCodes, StripHttpHeaders, Wrap,
};
