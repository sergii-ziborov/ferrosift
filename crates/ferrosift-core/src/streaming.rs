//! Running one operation over an input larger than memory.
//!
//! Every operation takes a whole [`Value`](ferrosift_model::Value) and returns
//! a whole one, which is the right shape for almost everything and the wrong
//! shape for the case this crate exists to serve: hashing a disk image,
//! encoding a firmware dump, XOR-ing a memory capture. The subject is not a
//! value anyone can hold.
//!
//! `StreamingSupport::Incremental` has been in the model since the beginning
//! and nothing declared it, because there was nothing to declare. This is the
//! contract that makes it mean something.
//!
//! # What is deliberately not here
//!
//! A streaming *executor*. Chaining incremental operations is a real thing to
//! want and a different problem — back-pressure, a chunk boundary that is not
//! the next operation's boundary, an operation in the middle of the chain that
//! cannot stream at all. One operation at a time is what a caller can use
//! today and what can be proven correct today: the property below says a
//! streamed answer *is* the buffered answer, and it is checked at every chunk
//! size rather than argued for.
//!
//! # Example
//!
//! ```
//! # use ferrosift_core::{OperationError, StreamSink};
//! /// Collects a streamed answer, which is what a test does and a caller does
//! /// not — the point is to write it out as it arrives.
//! struct Collect(Vec<u8>);
//!
//! impl StreamSink for Collect {
//!     fn write(&mut self, bytes: &[u8]) -> Result<(), OperationError> {
//!         self.0.extend_from_slice(bytes);
//!         Ok(())
//!     }
//! }
//! ```

use alloc::boxed::Box;
use alloc::vec::Vec;

use ferrosift_model::Arguments;

use crate::{OperationContext, OperationError};

/// Where a streamed answer goes.
///
/// Called many times as the answer is produced, in order. A sink that cannot
/// take the bytes says so with an [`OperationError`], which stops the run —
/// the same way an operation's own failure would.
pub trait StreamSink {
    /// Accepts the next part of the answer.
    ///
    /// # Errors
    ///
    /// Whatever the destination could not do.
    fn write(&mut self, bytes: &[u8]) -> Result<(), OperationError>;
}

/// Collects a streamed answer into memory.
///
/// For a caller who wants the streaming *input* and can hold the output — a
/// hash of a disk image is thirty-two bytes — and for the tests that check a
/// streamed answer against the buffered one.
#[derive(Debug, Default)]
pub struct CollectSink {
    bytes: Vec<u8>,
}

impl CollectSink {
    /// An empty collector.
    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// The answer so far.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Takes the answer, leaving the collector empty.
    #[must_use]
    pub fn take(self) -> Vec<u8> {
        self.bytes
    }
}

impl StreamSink for CollectSink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), OperationError> {
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

/// One operation, part way through an input.
///
/// The state between chunks. An implementation holds whatever it needs — a
/// digest's internal block, an encoder's partial group, a key position — and
/// nothing else: the whole point is that the memory is bounded by the
/// operation rather than by the subject.
///
/// Chunk boundaries are the caller's and carry no meaning. An implementation
/// that behaved differently for `push(a); push(b)` than for `push(ab)` would
/// be answering a question about how the file was read.
pub trait StreamSession {
    /// Consumes the next part of the input.
    ///
    /// # Errors
    ///
    /// As the operation's own [`Operation::execute`](crate::Operation::execute),
    /// plus whatever the sink refused.
    fn push(&mut self, chunk: &[u8], sink: &mut dyn StreamSink) -> Result<(), OperationError>;

    /// Ends the run, emitting whatever was held back.
    ///
    /// Boxed by value so a session cannot be used after it finishes — a digest
    /// has exactly one answer and a partial group is flushed exactly once.
    ///
    /// # Errors
    ///
    /// As [`Self::push`], plus anything only a complete input can detect —
    /// truncated padding, an unterminated escape.
    fn finish(self: Box<Self>, sink: &mut dyn StreamSink) -> Result<(), OperationError>;
}

/// An operation that can be run over an input it never holds whole.
///
/// Implemented by hand rather than derived, and declared in the specification
/// as [`StreamingSupport::Incremental`](ferrosift_model::StreamingSupport). The
/// two must agree: `tests/streaming.rs` fails an operation that declares the
/// support and offers no session, and one that offers a session whose answer
/// differs from `execute`'s at any chunk size.
pub trait Streamable {
    /// Starts a run, or `None` when these arguments cannot be streamed.
    ///
    /// Arguments may decide it. Hex encoding streams with any delimiter; a
    /// future operation might stream in one mode and not another, and `None`
    /// is how it says so without failing.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::InvalidArguments`] where the arguments are
    /// wrong rather than merely unstreamable — the same failure `execute`
    /// would give for them.
    fn start(
        &self,
        arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<Option<Box<dyn StreamSession + '_>>, OperationError>;
}

/// Runs `session` over `chunks`, writing the answer to `sink`.
///
/// A convenience for the common shape, and the shape the tests use. A caller
/// pulling from a file reads and pushes in their own loop instead; nothing
/// here needs to own the reading.
///
/// # Errors
///
/// Whatever the session or the sink refused.
pub fn drive<'a>(
    mut session: Box<dyn StreamSession + 'a>,
    chunks: impl IntoIterator<Item = &'a [u8]>,
    sink: &mut dyn StreamSink,
) -> Result<(), OperationError> {
    for chunk in chunks {
        session.push(chunk, sink)?;
    }
    session.finish(sink)
}
