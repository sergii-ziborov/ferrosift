//! Streaming sessions shared by the operations that offer one.
//!
//! `StreamingSupport::Incremental` had been in the model since the beginning
//! with nothing declaring it. These are the first implementations, and they
//! are the cases where the difference is not an optimisation but the whole
//! point: a digest of a disk image, an XOR over a memory capture. The subject
//! is larger than the machine and the answer is not.
//!
//! Every one is checked against the buffered path rather than trusted:
//! `tests/streaming.rs` runs each declared operation at eight chunk sizes over
//! several inputs and requires the same bytes as `execute` produced.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationError, StreamSession, StreamSink};

use crate::hex_util::to_hex_lower;

/// A digest, fed a chunk at a time.
///
/// One session for every fixed-output hash in the catalog, because `RustCrypto`'s
/// `DynDigest` is object-safe and every one of them implements it. The digest
/// holds a block and a state — tens of bytes — whatever the subject weighs.
#[cfg(feature = "hash")]
pub(crate) struct DigestSession {
    digest: Box<dyn digest::DynDigest + Send>,
}

#[cfg(feature = "hash")]
impl DigestSession {
    pub(crate) const fn new(digest: Box<dyn digest::DynDigest + Send>) -> Self {
        Self { digest }
    }
}

#[cfg(feature = "hash")]
impl StreamSession for DigestSession {
    fn push(&mut self, chunk: &[u8], _sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        // Nothing is written until the end: a hash has no partial answer, and
        // emitting one would be emitting something that is not the answer.
        self.digest.update(chunk);
        Ok(())
    }

    fn finish(self: Box<Self>, sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        let digest = self.digest.finalize();
        sink.write(to_hex_lower(&digest).as_bytes())
    }
}

/// A byte-wise transform with a repeating key.
///
/// XOR is the shape: each output byte depends on its input byte and on its
/// position, and on nothing else. The state is the position within the key,
/// which is why a chunk boundary anywhere produces the same answer.
pub(crate) struct KeyedByteSession {
    key: Vec<u8>,
    position: usize,
    /// Reused between chunks so a long run does not allocate per chunk.
    buffer: Vec<u8>,
}

impl KeyedByteSession {
    pub(crate) const fn new(key: Vec<u8>) -> Self {
        Self {
            key,
            position: 0,
            buffer: Vec::new(),
        }
    }
}

impl StreamSession for KeyedByteSession {
    fn push(&mut self, chunk: &[u8], sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        if self.key.is_empty() {
            // An empty key is the identity, which the buffered path also
            // answers rather than refusing.
            return sink.write(chunk);
        }
        self.buffer.clear();
        self.buffer.reserve(chunk.len());
        for byte in chunk {
            self.buffer.push(byte ^ self.key[self.position]);
            self.position = (self.position + 1) % self.key.len();
        }
        sink.write(&self.buffer)
    }

    fn finish(self: Box<Self>, _sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        // Nothing is held back: every byte was answered as it arrived.
        Ok(())
    }
}

/// Lower-case hexadecimal, a chunk at a time.
///
/// The contiguous form only — a delimiter or a line width makes the output
/// depend on the *position* of the last byte, which a session cannot know
/// until it ends. Those arguments answer `None` from `start` rather than
/// producing a different answer than the buffered path, which is the rule the
/// whole contract rests on.
pub(crate) struct HexSession {
    buffer: String,
}

impl HexSession {
    pub(crate) const fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl StreamSession for HexSession {
    fn push(&mut self, chunk: &[u8], sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        self.buffer.clear();
        self.buffer.reserve(chunk.len() * 2);
        self.buffer.push_str(&to_hex_lower(chunk));
        sink.write(self.buffer.as_bytes())
    }

    fn finish(self: Box<Self>, _sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        Ok(())
    }
}
