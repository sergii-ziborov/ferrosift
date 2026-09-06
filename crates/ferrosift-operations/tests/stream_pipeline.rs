//! The demo chain: Base64 Decode → Gunzip → SHA-256.
//!
//! Base64 and SHA-2 stream. Gunzip is the materialisation barrier until an
//! incremental inflater is wired: decoded gzip bytes are collected, inflated
//! once, then hashed incrementally. The streamed answer must still match a
//! fully buffered recipe.

#![cfg(all(feature = "hash", feature = "compression-deflate"))]

use ferrosift_core::{
    CollectSink, ExecutionBudget, NeverCancelled, Operation, OperationContext, StreamPipeline,
    Streamable, drive,
};
use ferrosift_model::{ArgumentValue, Arguments, CapabilitySet, TextEncoding, TextValue, Value};
use ferrosift_operations::{FromBase64, Gunzip, Gzip, Sha2};

fn context() -> OperationContext<'static> {
    OperationContext::new(
        ExecutionBudget::generous(),
        &NeverCancelled,
        CapabilitySet::new(),
    )
}

fn arguments(pairs: &[(&str, ArgumentValue)]) -> Arguments {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect()
}

#[test]
fn base64_gunzip_sha256_matches_buffered_with_gunzip_barrier() {
    let plaintext = b"ferrosift streaming demo payload";
    let mut ctx = context();
    let gzipped = Gzip::new()
        .execute(
            Value::Bytes(plaintext.to_vec()),
            &arguments(&[
                (
                    "compression_type",
                    ArgumentValue::Text("Dynamic Huffman Coding".to_owned()),
                ),
                ("filename", ArgumentValue::Text(String::new())),
                ("comment", ArgumentValue::Text(String::new())),
                ("include_file_checksum", ArgumentValue::Boolean(false)),
            ]),
            &mut ctx,
        )
        .expect("gzip");
    let Value::Bytes(gzip_bytes) = gzipped else {
        panic!("expected bytes");
    };

    let mut ctx = context();
    let encoded = ferrosift_operations::ToBase64::new()
        .execute(
            Value::Bytes(gzip_bytes),
            &arguments(&[("alphabet", ArgumentValue::Text("A-Za-z0-9+/=".to_owned()))]),
            &mut ctx,
        )
        .expect("base64");
    let Value::Text(b64) = encoded else {
        panic!("expected text");
    };

    // Buffered recipe path: Base64 → Gunzip → SHA-256.
    let decode_args = arguments(&[
        ("alphabet", ArgumentValue::Text("A-Za-z0-9+/=".to_owned())),
        ("remove_non_alphabet", ArgumentValue::Boolean(true)),
        ("strict", ArgumentValue::Boolean(false)),
    ]);
    let sha_args = arguments(&[
        ("size", ArgumentValue::Text("256".to_owned())),
        ("rounds_256", ArgumentValue::Integer(64)),
        ("rounds_512", ArgumentValue::Integer(160)),
    ]);
    let mut ctx = context();
    let decoded = FromBase64::new()
        .execute(
            Value::Text(TextValue {
                text: b64.text.clone(),
                encoding: TextEncoding::Utf8,
            }),
            &decode_args,
            &mut ctx,
        )
        .expect("decode");
    let mut ctx = context();
    let inflated = Gunzip::new()
        .execute(decoded, &Arguments::new(), &mut ctx)
        .expect("gunzip");
    let mut ctx = context();
    let buffered = Sha2::new()
        .execute(inflated, &sha_args, &mut ctx)
        .expect("sha2");
    let Value::Text(expected) = buffered else {
        panic!("expected hex text");
    };

    // Streamed path with Gunzip as the materialisation barrier.
    let start_ctx = context();
    let mut decode_session = FromBase64::new()
        .start(&decode_args, &start_ctx)
        .expect("start")
        .expect("session");
    let mut decoded_sink = CollectSink::new();
    for chunk in b64.text.as_bytes().chunks(7) {
        decode_session.push(chunk, &mut decoded_sink).expect("push");
    }
    decode_session.finish(&mut decoded_sink).expect("finish");

    let mut ctx = context();
    let inflated = Gunzip::new()
        .execute(
            Value::Bytes(decoded_sink.take()),
            &Arguments::new(),
            &mut ctx,
        )
        .expect("gunzip barrier");
    let Value::Bytes(inflated_bytes) = inflated else {
        panic!("expected bytes");
    };

    let start_ctx = context();
    let sha_session = Sha2::new()
        .start(&sha_args, &start_ctx)
        .expect("start")
        .expect("session");
    let mut hash_sink = CollectSink::new();
    drive(sha_session, inflated_bytes.chunks(5), &mut hash_sink).expect("sha stream");
    let actual = String::from_utf8(hash_sink.take()).expect("utf8");
    assert_eq!(actual, expected.text);
}

#[test]
fn stream_pipeline_chains_base64_into_sha2_without_caller_buffer() {
    // A shorter chain that is fully incremental end-to-end (no gunzip).
    let payload = b"pipeline-only";
    let mut ctx = context();
    let encoded = ferrosift_operations::ToBase64::new()
        .execute(
            Value::Bytes(payload.to_vec()),
            &arguments(&[("alphabet", ArgumentValue::Text("A-Za-z0-9+/=".to_owned()))]),
            &mut ctx,
        )
        .expect("encode");
    let Value::Text(b64) = encoded else {
        panic!("expected text");
    };

    let decode_args = arguments(&[
        ("alphabet", ArgumentValue::Text("A-Za-z0-9+/=".to_owned())),
        ("remove_non_alphabet", ArgumentValue::Boolean(true)),
        ("strict", ArgumentValue::Boolean(false)),
    ]);
    let sha_args = arguments(&[
        ("size", ArgumentValue::Text("256".to_owned())),
        ("rounds_256", ArgumentValue::Integer(64)),
        ("rounds_512", ArgumentValue::Integer(160)),
    ]);

    let mut ctx = context();
    let decoded = FromBase64::new()
        .execute(
            Value::Text(TextValue {
                text: b64.text.clone(),
                encoding: TextEncoding::Utf8,
            }),
            &decode_args,
            &mut ctx,
        )
        .expect("decode");
    let mut ctx = context();
    let expected = Sha2::new()
        .execute(decoded, &sha_args, &mut ctx)
        .expect("sha2");
    let Value::Text(expected) = expected else {
        panic!("expected hex");
    };

    let start_ctx = context();
    let mut pipeline = StreamPipeline::new();
    pipeline.push_stage(
        FromBase64::new()
            .start(&decode_args, &start_ctx)
            .expect("start")
            .expect("session"),
    );
    pipeline.push_stage(
        Sha2::new()
            .start(&sha_args, &start_ctx)
            .expect("start")
            .expect("session"),
    );
    let mut sink = CollectSink::new();
    pipeline
        .drive(b64.text.as_bytes().chunks(3), &mut sink)
        .expect("pipeline");
    assert_eq!(String::from_utf8(sink.take()).expect("utf8"), expected.text);
}
