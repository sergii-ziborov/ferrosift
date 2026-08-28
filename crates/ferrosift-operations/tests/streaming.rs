//! A streamed answer is the buffered answer.
//!
//! That sentence is the whole contract, and it is the only thing that makes
//! streaming worth having: a caller who cannot hold a disk image in memory
//! still has to get the same bytes as one who can. An implementation that was
//! *nearly* right — a partial group flushed twice, a key position reset at a
//! chunk boundary, a digest finalised early — would look correct on the one
//! chunk size anyone tried by hand.
//!
//! So it is checked at every chunk size that matters: one byte at a time,
//! sizes that do and do not divide the input, sizes larger than the input, and
//! all at once. Chunk boundaries are the caller's and carry no meaning; an
//! operation that answered differently for `push(a); push(b)` than for
//! `push(ab)` would be answering a question about how the file was read.

#![cfg(feature = "hash")]

use ferrosift_core::{
    CollectSink, ExecutionBudget, NeverCancelled, Operation, OperationContext, Streamable, drive,
};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationSpec, StreamingSupport, Value,
};
use ferrosift_operations::{Sha2, ToHex, Xor};

/// Sizes that break an implementation in different ways.
///
/// One is the hardest case for anything holding a partial group. Three and
/// seven do not divide the inputs below. Sixty-four is a digest's block size.
/// The last two are larger than the input, so `push` is called once.
const CHUNKS: &[usize] = &[1, 2, 3, 4, 7, 16, 64, 1024];

fn context() -> OperationContext<'static> {
    OperationContext::new(
        ExecutionBudget::generous(),
        &NeverCancelled,
        CapabilitySet::new(),
    )
}

/// The buffered answer, as bytes.
fn buffered(operation: &dyn Operation, arguments: &Arguments, input: &[u8]) -> Vec<u8> {
    let mut context = context();
    let value = operation
        .execute(Value::Bytes(input.to_vec()), arguments, &mut context)
        .expect("the buffered path answers");
    match value {
        Value::Bytes(bytes) => bytes,
        Value::Text(text) => text.text.into_bytes(),
        other => panic!("unexpected output kind {:?}", other.kind()),
    }
}

/// The streamed answer at one chunk size.
fn streamed<S: Streamable>(
    operation: &S,
    arguments: &Arguments,
    input: &[u8],
    chunk: usize,
) -> Option<Vec<u8>> {
    let context = context();
    let session = operation
        .start(arguments, &context)
        .expect("starting must not fail for arguments the buffered path accepts")?;
    let mut sink = CollectSink::new();
    drive(session, input.chunks(chunk.max(1)), &mut sink).expect("the streamed path answers");
    Some(sink.take())
}

/// Every chunk size must give the buffered answer.
fn agrees<S: Streamable + Operation>(operation: &S, arguments: &Arguments, input: &[u8]) {
    let expected = buffered(operation, arguments, input);
    for chunk in CHUNKS {
        let actual = streamed(operation, arguments, input, *chunk)
            .expect("this operation offers a session for these arguments");
        assert_eq!(
            actual,
            expected,
            "{} disagreed at chunk size {chunk} over {} bytes",
            operation.spec().id.as_str(),
            input.len()
        );
    }
    // And with no chunks at all, which is a different path from a chunk of
    // zero bytes: `finish` runs having seen nothing.
    let context = context();
    let session = operation
        .start(arguments, &context)
        .expect("starting must not fail")
        .expect("a session");
    let mut sink = CollectSink::new();
    drive(session, core::iter::empty(), &mut sink).expect("an empty run answers");
    if input.is_empty() {
        assert_eq!(sink.take(), expected, "an empty input through no chunks");
    }
}

/// Inputs of several lengths, including the ones that divide nothing.
fn inputs() -> Vec<Vec<u8>> {
    vec![
        Vec::new(),
        vec![0x00],
        vec![0xff; 2],
        (0..=255_u8).collect(),
        (0..1000_u32).map(|value| (value % 251) as u8).collect(),
        // A length that is prime, so no chunk size above divides it.
        (0..1013_u32).map(|value| (value % 97) as u8).collect(),
    ]
}

fn arguments(pairs: &[(&str, ArgumentValue)]) -> Arguments {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect()
}

#[test]
fn a_streamed_digest_is_the_buffered_digest() {
    let operation = Sha2::new();
    for size in ["224", "256", "384", "512", "512/224", "512/256"] {
        let arguments = arguments(&[
            ("size", ArgumentValue::Text(size.to_owned())),
            ("rounds_256", ArgumentValue::Integer(64)),
            ("rounds_512", ArgumentValue::Integer(160)),
        ]);
        for input in inputs() {
            agrees(&operation, &arguments, &input);
        }
    }
}

#[test]
fn a_streamed_hex_encoding_is_the_buffered_one() {
    let operation = ToHex::new();
    let arguments = arguments(&[
        ("delimiter", ArgumentValue::Text("None".to_owned())),
        ("bytes_per_line", ArgumentValue::Integer(0)),
    ]);
    for input in inputs() {
        agrees(&operation, &arguments, &input);
    }
}

#[test]
fn a_streamed_xor_is_the_buffered_one() {
    let operation = Xor::new();
    for key in ["", "41", "cafebabe", "0102030405060708090a0b"] {
        let arguments = arguments(&[
            (
                "key",
                ArgumentValue::Map(arguments(&[
                    ("option", ArgumentValue::Text("Hex".to_owned())),
                    ("string", ArgumentValue::Text(key.to_owned())),
                ])),
            ),
            ("scheme", ArgumentValue::Text("Standard".to_owned())),
            ("null_preserving", ArgumentValue::Boolean(false)),
        ]);
        for input in inputs() {
            agrees(&operation, &arguments, &input);
        }
    }
}

/// Arguments an operation cannot stream answer `None`, and say so cleanly.
///
/// The alternative is worse than not streaming: a session that streamed the
/// delimited form would have to guess where the last byte is, and would be
/// wrong about the trailing delimiter every time. `None` sends the caller to
/// the buffered path, which is correct and merely needs the memory.
#[test]
fn an_unstreamable_argument_declines_rather_than_approximating() {
    let operation = ToHex::new();
    let context = context();

    for (delimiter, line) in [("Space", 0), ("None", 16), ("Comma", 0), ("0x", 8)] {
        let arguments = arguments(&[
            ("delimiter", ArgumentValue::Text(delimiter.to_owned())),
            ("bytes_per_line", ArgumentValue::Integer(line)),
        ]);
        assert!(
            operation
                .start(&arguments, &context)
                .expect("valid arguments do not fail")
                .is_none(),
            "delimiter {delimiter:?} with {line} bytes per line should decline"
        );
    }

    // And the one form it does stream still does.
    let contiguous = arguments(&[
        ("delimiter", ArgumentValue::Text("None".to_owned())),
        ("bytes_per_line", ArgumentValue::Integer(0)),
    ]);
    assert!(
        operation
            .start(&contiguous, &context)
            .expect("valid arguments do not fail")
            .is_some()
    );
}

/// Arguments the buffered path refuses, the streaming path refuses too.
#[test]
fn a_refusal_is_the_same_refusal() {
    let operation = Sha2::new();
    let starting = context();
    // A reduced-round SHA-2 is a different function, and both paths say so.
    let reduced = arguments(&[
        ("size", ArgumentValue::Text("256".to_owned())),
        ("rounds_256", ArgumentValue::Integer(32)),
        ("rounds_512", ArgumentValue::Integer(160)),
    ]);
    let mut buffered_context = context();
    let buffered = operation
        .execute(Value::Bytes(Vec::new()), &reduced, &mut buffered_context)
        .expect_err("a reduced-round digest is refused");
    let Err(streamed) = operation.start(&reduced, &starting) else {
        panic!("streaming must refuse a reduced-round digest too");
    };
    assert_eq!(buffered, streamed);
}

/// The declaration and the implementation must agree.
///
/// An operation that says `Incremental` and offers no session would have a
/// specification that lies, which is worse than one that says `Buffered` — a
/// caller reads the declaration to decide whether to bother.
#[test]
fn every_declared_incremental_operation_offers_a_session() {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let declared: Vec<&OperationSpec> = registry
        .catalog()
        .filter(|spec| matches!(spec.streaming, StreamingSupport::Incremental))
        .collect();

    // The list is short and named, so it stays readable and so that adding one
    // is a deliberate act. Streaming is implemented operation by operation;
    // this is where v1 got to.
    let ids: Vec<&str> = declared.iter().map(|spec| spec.id.as_str()).collect();
    assert_eq!(
        ids,
        ["encoding.hex.encode@1", "hash.sha2@1", "logic.xor@1"],
        "every operation declaring incremental streaming should be named here"
    );

    // Each one really does offer a session under its own defaults.
    let context = context();
    for spec in declared {
        let defaults: Arguments = spec
            .arguments
            .iter()
            .filter_map(|argument| {
                argument
                    .default
                    .clone()
                    .map(|value| (argument.name.clone(), value))
            })
            .collect();
        let offered = match spec.id.as_str() {
            "encoding.hex.encode@1" => {
                // The default delimiter is `Space`, which is the form that
                // declines — so this asks with the one it streams.
                let contiguous = arguments(&[
                    ("delimiter", ArgumentValue::Text("None".to_owned())),
                    ("bytes_per_line", ArgumentValue::Integer(0)),
                ]);
                ToHex::new()
                    .start(&contiguous, &context)
                    .map(|s| s.is_some())
            }
            "hash.sha2@1" => Sha2::new().start(&defaults, &context).map(|s| s.is_some()),
            "logic.xor@1" => Xor::new().start(&defaults, &context).map(|s| s.is_some()),
            other => panic!("{other} declares incremental streaming and is not checked here"),
        };
        assert_eq!(
            offered,
            Ok(true),
            "{} declares incremental streaming and offered no session",
            spec.id.as_str()
        );
    }
}
