//! Where the bytes a pattern describes come from.

use alloc::string::String;
use alloc::vec::Vec;

/// The widest scalar the language has, and so the largest single read.
///
/// `u128` and `s128` are sixteen bytes; nothing reads more than one scalar at
/// a time. Fixing the ceiling here is what lets a read go through a stack
/// buffer, so a source never has to allocate to answer one and the trait stays
/// usable on a target with no allocator to spare.
pub const MAX_SCALAR_BYTES: usize = 16;

/// Why a source could not serve a read.
///
/// Deliberately not an `io::Error`: this crate is `no_std` and the sources that
/// matter most are the ones that are not files — a memory-mapped region, a
/// device window, a decompressor's output, a block cache over a disk. What
/// every one of them can produce is a sentence, so a sentence is what the trait
/// asks for, and the evaluator wraps it in a `pattern.eval.source_failed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceError {
    detail: String,
}

impl SourceError {
    /// Records why the read could not be served.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// The explanation, for a diagnostic rather than for control flow.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Bytes a pattern can be evaluated against.
///
/// Evaluation used to take a `&[u8]`, which quietly required the whole subject
/// to be in memory at once — fine for a packet and wrong for the case this
/// engine exists for, which is describing a disk image or a firmware dump
/// larger than the machine reading it. A pattern touches a vanishing fraction
/// of what it describes: every read here is one scalar, at a known offset, of
/// at most [`MAX_SCALAR_BYTES`].
///
/// The bounds check stays with the evaluator, not with the implementor. A read
/// that would leave the data is refused before it is asked for, so
/// `read_exact_at` is only ever called for a range this source said it has —
/// and an implementation is still free to fail, because a range being in range
/// is not a promise that the medium behind it is still there.
///
/// # Example
///
/// ```
/// use ferrosift_pattern::{ByteSource, SourceError};
///
/// /// Two buffers read as one, with no copy joining them.
/// struct Pair<'a>(&'a [u8], &'a [u8]);
///
/// impl ByteSource for Pair<'_> {
///     fn len(&self) -> u64 {
///         (self.0.len() + self.1.len()) as u64
///     }
///
///     fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
///         for (step, slot) in into.iter_mut().enumerate() {
///             let at = offset as usize + step;
///             *slot = *self
///                 .0
///                 .get(at)
///                 .or_else(|| self.1.get(at - self.0.len()))
///                 .ok_or_else(|| SourceError::new("past the end of both halves"))?;
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait ByteSource {
    /// How many bytes this source can serve, counted from zero.
    fn len(&self) -> u64;

    /// Whether the source has no bytes at all.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `into` from `offset`, or reports why it could not.
    ///
    /// `into` is never longer than [`MAX_SCALAR_BYTES`], and the evaluator has
    /// already checked that `offset + into.len()` is within [`len`](Self::len).
    ///
    /// # Errors
    ///
    /// Returns a [`SourceError`] when the underlying medium cannot serve the
    /// range, whatever the length said.
    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError>;
}

/// The ordinary case: bytes already in memory.
impl ByteSource for [u8] {
    fn len(&self) -> u64 {
        // A slice length always fits: `usize` is at most sixty-four bits on
        // every target this builds for, and `u64` holds all of it.
        self.len() as u64
    }

    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
        let start =
            usize::try_from(offset).map_err(|_| SourceError::new("offset is past this slice"))?;
        let end = start
            .checked_add(into.len())
            .ok_or_else(|| SourceError::new("read length overflows"))?;
        let bytes = self
            .get(start..end)
            .ok_or_else(|| SourceError::new("read extends past the end of the slice"))?;
        into.copy_from_slice(bytes);
        Ok(())
    }
}

/// A literal buffer, so `evaluate(&pattern, b"...", ..)` keeps working.
impl<const N: usize> ByteSource for [u8; N] {
    fn len(&self) -> u64 {
        N as u64
    }

    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
        <[u8] as ByteSource>::read_exact_at(self.as_slice(), offset, into)
    }
}

impl ByteSource for Vec<u8> {
    fn len(&self) -> u64 {
        self.len() as u64
    }

    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
        <[u8] as ByteSource>::read_exact_at(self.as_slice(), offset, into)
    }
}

/// So a caller may pass a reference to a source it does not own.
impl<S: ByteSource + ?Sized> ByteSource for &S {
    fn len(&self) -> u64 {
        (**self).len()
    }

    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
        (**self).read_exact_at(offset, into)
    }
}
