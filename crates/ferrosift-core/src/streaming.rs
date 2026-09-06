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
//! # Pipelines
//!
//! [`StreamPipeline`] chains several sessions so the bytes leaving one stage
//! enter the next without a full intermediate buffer owned by the caller. An
//! operation that cannot stream is still a materialisation barrier outside
//! this type: the host buffers, then continues.
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
    ) -> Result<Option<Box<dyn StreamSession>>, OperationError>;
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

/// Runs several incremental sessions as a pipeline.
///
/// Output of stage *n* is pushed into stage *n + 1*; the last stage writes to
/// `sink`. Stages are taken out of the vector while they run so nested pushes
/// do not fight the borrow checker.
#[derive(Default)]
pub struct StreamPipeline {
    stages: Vec<Option<Box<dyn StreamSession>>>,
}

impl StreamPipeline {
    /// Empty pipeline; [`Self::push_stage`] adds sessions in order.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one incremental session.
    pub fn push_stage(&mut self, session: Box<dyn StreamSession>) {
        self.stages.push(Some(session));
    }

    /// Feeds `chunks` through every stage into `sink`.
    ///
    /// # Errors
    ///
    /// Whatever a session or the sink refused.
    pub fn drive<'a>(
        &mut self,
        chunks: impl IntoIterator<Item = &'a [u8]>,
        sink: &mut dyn StreamSink,
    ) -> Result<(), OperationError> {
        for chunk in chunks {
            self.feed(0, chunk, sink)?;
        }
        self.finish_from(0, sink)
    }

    fn feed(
        &mut self,
        index: usize,
        chunk: &[u8],
        sink: &mut dyn StreamSink,
    ) -> Result<(), OperationError> {
        if index >= self.stages.len() {
            return sink.write(chunk);
        }
        let mut session = self.stages[index]
            .take()
            .expect("pipeline stage should be present");
        let result = {
            let mut bridge = PipelineBridge {
                pipeline: self,
                next: index + 1,
                sink,
            };
            session.push(chunk, &mut bridge)
        };
        self.stages[index] = Some(session);
        result
    }

    fn finish_from(
        &mut self,
        index: usize,
        sink: &mut dyn StreamSink,
    ) -> Result<(), OperationError> {
        if index >= self.stages.len() {
            return Ok(());
        }
        let session = self.stages[index]
            .take()
            .expect("pipeline stage should be present");
        {
            let mut bridge = PipelineBridge {
                pipeline: self,
                next: index + 1,
                sink,
            };
            session.finish(&mut bridge)?;
        }
        self.finish_from(index + 1, sink)
    }
}

struct PipelineBridge<'a> {
    pipeline: &'a mut StreamPipeline,
    next: usize,
    sink: &'a mut dyn StreamSink,
}

impl StreamSink for PipelineBridge<'_> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), OperationError> {
        self.pipeline.feed(self.next, bytes, self.sink)
    }
}

/// Convenience: drive a pipeline and collect the answer.
///
/// # Errors
///
/// Whatever [`StreamPipeline::drive`] refused.
pub fn drive_pipeline_collect<'a>(
    pipeline: &mut StreamPipeline,
    chunks: impl IntoIterator<Item = &'a [u8]>,
) -> Result<Vec<u8>, OperationError> {
    let mut sink = CollectSink::new();
    pipeline.drive(chunks, &mut sink)?;
    Ok(sink.take())
}
