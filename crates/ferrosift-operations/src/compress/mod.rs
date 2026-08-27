mod limits;

#[cfg(feature = "compression-bzip2")]
mod bzip2;
#[cfg(feature = "compression-deflate")]
mod codec;
mod container;
#[cfg(feature = "compression-deflate")]
mod deflate;

#[cfg(feature = "compression-bzip2")]
pub use container::{Bzip2Compress, Bzip2Decompress};
#[cfg(feature = "compression-deflate")]
pub use container::{Gunzip, Gzip};
#[cfg(feature = "compression-deflate")]
pub use deflate::{RawDeflate, RawInflate, ZlibDeflate, ZlibInflate};
