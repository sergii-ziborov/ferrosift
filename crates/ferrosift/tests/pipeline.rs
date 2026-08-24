//! Pipeline behaviour: composition, input adaptation, and pattern hand-off.

use ferrosift::prelude::*;
use ferrosift::{ExecutionBudget, TextEncoding, TextValue, default_budget, registry};

#[test]
fn a_single_step_decodes_bytes() {
    let output = pipeline()
        .from_base64()
        .run_bytes(b"Zm9v")
        .expect("decode succeeds");
    assert_eq!(output, b"foo");
}

#[test]
fn steps_compose_in_declaration_order() {
    let output = pipeline()
        .from_base64()
        .to_hex()
        .run_bytes(b"Zm9v")
        .expect("compose succeeds");
    assert_eq!(output, b"66 6f 6f");
}

#[test]
fn round_trips_return_the_original_bytes() {
    let original = [0x00_u8, 0x46, 0xff, 0x10];
    let output = pipeline()
        .to_base64()
        .from_base64()
        .run_bytes(&original)
        .expect("round trip succeeds");
    assert_eq!(output, original);
}

#[test]
fn text_entry_point_returns_text() {
    let output = pipeline().to_hex().run_text("Hi").expect("encode succeeds");
    assert_eq!(output, "48 69");
}

#[test]
fn bytes_are_adapted_to_a_text_taking_first_step() {
    // `from_base64` requires text; the caller may still hand over bytes.
    let output = pipeline()
        .from_base64()
        .run_bytes(b"aGVsbG8=")
        .expect("adaptation succeeds");
    assert_eq!(output, b"hello");
}

#[test]
fn text_is_adapted_to_a_byte_taking_first_step() {
    // `to_hex` requires bytes; the caller may still hand over text.
    let output = pipeline()
        .to_hex()
        .run_text("Hi")
        .expect("adaptation succeeds");
    assert_eq!(output, "48 69");
}

#[test]
fn xor_applies_a_repeating_key() {
    let output = pipeline()
        .xor(&[0x0f])
        .run_bytes(&[0x00, 0xff])
        .expect("xor succeeds");
    assert_eq!(output, [0x0f, 0xf0]);
}

#[test]
fn hashes_produce_lower_case_hexadecimal_text() {
    let output = pipeline().md5().run_bytes(b"hello").expect("hash succeeds");
    assert_eq!(output, b"5d41402abc4b2a76b9719d911017c592");
}

#[test]
fn the_escape_hatch_reaches_operations_without_sugar() {
    let output = pipeline()
        .step("encoding.rot13@1", Arguments::new())
        .run_bytes(b"Hello")
        .expect("rot13 succeeds");
    assert_eq!(output, b"Uryyb");
}

#[test]
fn identity_and_convenience_steps_are_plain_canonical_ids() {
    let built = pipeline()
        .identity()
        .from_base32()
        .from_base58()
        .from_base85()
        .from_hex()
        .url_decode()
        .url_encode()
        .to_base64()
        .rot13()
        .sha1()
        .sha2();
    assert_eq!(
        built.operations(),
        [
            "core.identity@1",
            "encoding.base32.decode@1",
            "encoding.base58.decode@1",
            "encoding.base85.decode@1",
            "encoding.hex.decode@1",
            "encoding.url.decode@1",
            "encoding.url.encode@1",
            "encoding.base64.encode@1",
            "encoding.rot13@1",
            "hash.sha1@1",
            "hash.sha2@1",
        ]
    );
}

#[test]
fn compression_steps_use_canonical_ids() {
    let built = pipeline()
        .gzip()
        .gunzip()
        .zlib_inflate()
        .raw_inflate()
        .bzip2_decompress();
    assert_eq!(
        built.operations(),
        [
            "compression.gzip@1",
            "compression.gunzip@1",
            "compression.zlib.inflate@1",
            "compression.raw.inflate@1",
            "compression.bzip2.decompress@1",
        ]
    );
}

#[test]
fn a_gzip_round_trip_survives_the_pipeline() {
    let compressed = pipeline()
        .gzip()
        .run_bytes(b"ferrosift ferrosift ferrosift")
        .expect("compress succeeds");
    let restored = pipeline()
        .gunzip()
        .run_bytes(&compressed)
        .expect("decompress succeeds");
    assert_eq!(restored, b"ferrosift ferrosift ferrosift");
}

#[test]
fn an_empty_pipeline_returns_its_input_untouched() {
    let output = pipeline().run_bytes(b"unchanged").expect("passes through");
    assert_eq!(output, b"unchanged");

    let value = pipeline()
        .run(Value::Bytes(vec![1, 2, 3]))
        .expect("passes through");
    assert_eq!(value, Value::Bytes(vec![1, 2, 3]));
}

#[test]
fn transform_then_parse_reads_the_decoded_bytes() {
    // "Q0FGRQ==" decodes to the ASCII bytes "CAFE".
    let nodes = pipeline()
        .from_base64()
        .run_pattern(
            "struct Head { be u16 first; be u16 second; };
             Head head @ 0x00;",
            b"Q0FGRQ==",
        )
        .expect("transform then parse succeeds");

    let head = &nodes[0];
    assert_eq!(
        head.child("first").expect("field").value,
        NodeValue::Unsigned(0x4341)
    );
    assert_eq!(
        head.child("second").expect("field").value,
        NodeValue::Unsigned(0x4645)
    );
    assert_eq!((head.offset, head.size), (0, 4));
}

#[test]
fn pattern_evaluation_bounds_are_configurable() {
    let options = EvalOptions {
        max_nodes: 2,
        ..EvalOptions::default()
    };
    let error = pipeline()
        .run_pattern_with("u8 many[32] @ 0;", &[0; 32], &options)
        .expect_err("the node budget must stop evaluation");
    assert_eq!(error.code(), "pattern.eval.node_budget_exceeded");
}

#[test]
fn the_default_budget_is_bounded_and_overridable() {
    let default = default_budget();
    assert_eq!(default.max_steps, 256);
    assert_eq!(default.max_input_bytes, 16 * 1024 * 1024);

    let tight = ExecutionBudget {
        max_output_bytes: 2,
        ..default
    };
    let error = pipeline()
        .budget(tight)
        .to_hex()
        .run_bytes(b"too long for two bytes")
        .expect_err("the output budget must stop execution");
    assert_eq!(error.code(), "core.operation.output_limit_exceeded");
}

#[test]
fn the_registry_exposes_every_built_in_operation() {
    let registry = registry().expect("registry validates");
    assert!(registry.len() >= 68);
    assert!(
        registry
            .catalog()
            .any(|spec| spec.id.as_str() == "encoding.base64.decode@1")
    );
}

#[test]
fn a_pipeline_can_be_cloned_and_reused() {
    let base = pipeline().from_base64();
    let first = base.clone().run_bytes(b"Zm9v").expect("first run");
    let second = base.to_hex().run_bytes(b"Zm9v").expect("second run");
    assert_eq!(first, b"foo");
    assert_eq!(second, b"66 6f 6f");
}

#[test]
fn typed_values_pass_through_run_unchanged() {
    let value = Value::Text(TextValue {
        text: "Zm9v".into(),
        encoding: TextEncoding::Utf8,
    });
    let output = pipeline().from_base64().run(value).expect("run succeeds");
    assert_eq!(output, Value::Bytes(b"foo".to_vec()));
}
