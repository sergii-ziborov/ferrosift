//! LZNT1, the compression behind `RtlDecompressBuffer`.

mod codec;
mod operation;

pub use operation::Lznt1Decompress;
