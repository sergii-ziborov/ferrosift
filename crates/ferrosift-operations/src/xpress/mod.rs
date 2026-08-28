//! XPRESS, the compression behind `RtlDecompressBuffer`'s other two formats.
//!
//! Two of the three operations 11.4 introduced, so both are aliased from that
//! profile onward and 11.3 answers to neither. Decompression only, in both
//! directions of the pair: the reference has no XPRESS compressor either, and
//! the formats exist here to *read* what Windows produced — WIM images, WOF
//! compressed files, and the shadow-copy and hibernation blobs a forensic
//! recipe starts from.
//!
//! Neighbour to [`crate::lznt1`] rather than to [`crate::compress`]: these are
//! self-contained decoders with no external crate behind them, so they reach
//! every target the core does and carry no feature gate.

mod codec;
mod operation;

pub use operation::{XpressDecompress, XpressHuffmanDecompress};
