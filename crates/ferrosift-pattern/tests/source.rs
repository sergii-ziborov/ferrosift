//! Evaluating against bytes that are not a buffer.
//!
//! The engine used to take a `&[u8]`, which quietly required the whole subject
//! in memory. That is the wrong shape for what a hex pattern is *for*: a disk
//! image, a firmware dump, a memory capture. A pattern reads a vanishing
//! fraction of what it describes — one scalar per field, at a known offset —
//! so the whole was never needed, only the bytes named.
//!
//! These tests hold the three things that makes true. A source may be larger
//! than memory, may be discontiguous, and may fail; and the bounds check stays
//! with the evaluator rather than being delegated to whoever implements the
//! trait.

use core::cell::Cell;

use ferrosift_pattern::{ByteSource, EvalOptions, NodeValue, SourceError, evaluate_with};

/// Four gibibytes that were never allocated.
///
/// Each byte is derived from its own offset, so the source can answer any read
/// in constant time and constant space. If evaluation needed the subject in
/// memory this test could not exist — which is the point of it.
struct Synthesised {
    len: u64,
    reads: Cell<u32>,
}

impl Synthesised {
    fn new(len: u64) -> Self {
        Self {
            len,
            reads: Cell::new(0),
        }
    }

    /// The byte this source holds at `offset`.
    fn byte_at(offset: u64) -> u8 {
        // Arbitrary but position-dependent, so a read at the wrong offset
        // produces the wrong answer rather than the same one.
        (offset.wrapping_mul(31).wrapping_add(7) & 0xff) as u8
    }
}

impl ByteSource for Synthesised {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
        self.reads.set(self.reads.get() + 1);
        for (step, slot) in into.iter_mut().enumerate() {
            *slot = Self::byte_at(offset + step as u64);
        }
        Ok(())
    }
}

#[test]
fn a_source_may_be_larger_than_memory() {
    let source = Synthesised::new(4 * 1024 * 1024 * 1024);
    let pattern = ferrosift_pattern::parse(
        "struct Marker { be u32 first; be u16 second; };
         Marker marker @ 0xC0000000;",
    )
    .expect("pattern parses");

    let nodes = evaluate_with(&pattern, &source, &EvalOptions::default())
        .expect("a placement three gibibytes in is an ordinary read");

    let expected_first = u32::from_be_bytes([
        Synthesised::byte_at(0xC000_0000),
        Synthesised::byte_at(0xC000_0001),
        Synthesised::byte_at(0xC000_0002),
        Synthesised::byte_at(0xC000_0003),
    ]);
    let first = nodes[0].child("first").expect("field");
    assert_eq!(first.value, NodeValue::Unsigned(u128::from(expected_first)));
    assert_eq!((first.offset, first.size), (0xC000_0000, 4));

    let second = nodes[0].child("second").expect("field");
    assert_eq!((second.offset, second.size), (0xC000_0004, 2));

    // Two fields, two reads. Nothing was fetched speculatively and nothing was
    // fetched twice.
    assert_eq!(source.reads.get(), 2);
}

#[test]
fn the_evaluator_checks_the_bounds_rather_than_the_source() {
    // A read past the declared length must be refused *before* the source is
    // asked, so an implementation never has to defend itself against a range
    // the evaluator should have rejected. `Synthesised` answers any offset it
    // is given, so if it were consulted here the read would succeed.
    let source = Synthesised::new(4);
    let pattern = ferrosift_pattern::parse("be u32 value @ 0x02;").expect("pattern parses");

    let error = evaluate_with(&pattern, &source, &EvalOptions::default())
        .expect_err("four bytes at offset two leave a four-byte source");
    assert_eq!(error.code(), "pattern.eval.out_of_bounds");
    assert_eq!(source.reads.get(), 0, "the source must not have been asked");
}

/// Two buffers read as one, with no copy joining them.
struct Split<'a>(&'a [u8], &'a [u8]);

impl ByteSource for Split<'_> {
    fn len(&self) -> u64 {
        (self.0.len() + self.1.len()) as u64
    }

    fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
        let base = usize::try_from(offset)
            .map_err(|_| SourceError::new("offset is past what this pair can address"))?;
        for (step, slot) in into.iter_mut().enumerate() {
            let at = base + step;
            *slot = *self
                .0
                .get(at)
                .or_else(|| self.1.get(at.wrapping_sub(self.0.len())))
                .ok_or_else(|| SourceError::new("past the end of both halves"))?;
        }
        Ok(())
    }
}

#[test]
fn a_discontiguous_source_answers_what_the_join_would() {
    let pattern = ferrosift_pattern::parse(
        "struct Header { be u16 magic; u8 version; be u32 length; };
         Header header @ 0x00;",
    )
    .expect("pattern parses");
    let whole = [0xca, 0xfe, 0x03, 0x00, 0x00, 0x10, 0x00];

    // The same bytes, split across the boundary of a field rather than between
    // two of them: the `u32` here begins in the first half and ends in the
    // second, which is where a source that only served whole chunks would come
    // apart.
    let split = Split(&whole[..4], &whole[4..]);

    let joined = evaluate_with(&pattern, whole.as_slice(), &EvalOptions::default())
        .expect("the buffer evaluates");
    let apart = evaluate_with(&pattern, &split, &EvalOptions::default())
        .expect("the split source evaluates");

    assert_eq!(joined, apart);
    let length = apart[0].child("length").expect("field");
    assert_eq!(length.value, NodeValue::Unsigned(0x0000_1000));
}

/// A source whose medium has gone away.
struct Broken;

impl ByteSource for Broken {
    fn len(&self) -> u64 {
        1024
    }

    fn read_exact_at(&self, _offset: u64, _into: &mut [u8]) -> Result<(), SourceError> {
        Err(SourceError::new("the device stopped responding"))
    }
}

#[test]
fn a_source_that_fails_is_not_reported_as_a_malformed_pattern() {
    let pattern = ferrosift_pattern::parse("be u32 value @ 0x00;").expect("pattern parses");
    let error = evaluate_with(&pattern, &Broken, &EvalOptions::default())
        .expect_err("the source declines every read");

    // A distinct code, because the two failures have nothing to do with each
    // other: `out_of_bounds` says the pattern asked for bytes that are not
    // there, and this says the bytes are there and could not be fetched.
    // Reporting both alike would make a disk error look like a bad pattern.
    assert_eq!(error.code(), "pattern.eval.source_failed");
    assert!(
        error.detail().contains("device stopped responding"),
        "the source's own explanation should survive: {}",
        error.detail()
    );
}

#[test]
fn a_slice_is_a_source_and_evaluate_is_this_over_one() {
    // `evaluate` is defined as `evaluate_with` over a slice, and the two must
    // stay indistinguishable — otherwise the older entry point becomes a
    // second implementation with its own behaviour.
    let pattern = ferrosift_pattern::parse(
        "struct Row { u8 kind; le u16 count; };
         Row rows[3] @ 0x00;",
    )
    .expect("pattern parses");
    let data: [u8; 9] = [1, 0x10, 0x00, 2, 0x20, 0x00, 3, 0x30, 0x00];

    let through_slice =
        ferrosift_pattern::evaluate(&pattern, &data, &EvalOptions::default()).expect("evaluates");
    let through_source =
        evaluate_with(&pattern, &data, &EvalOptions::default()).expect("evaluates");
    let through_vec = evaluate_with(&pattern, &data.to_vec(), &EvalOptions::default())
        .expect("a Vec is a source too");

    assert_eq!(through_slice, through_source);
    assert_eq!(through_slice, through_vec);
}

#[test]
fn an_empty_source_says_so() {
    let empty: [u8; 0] = [];
    assert!(ByteSource::is_empty(&empty));
    assert_eq!(ByteSource::len(&empty), 0);
    assert!(!ByteSource::is_empty(&[7_u8; 1]));
    // The provided method reads the implementor's own length rather than a
    // buffer, so a source with nothing behind it yet still reports its size.
    assert!(!ByteSource::is_empty(&Broken));
}
