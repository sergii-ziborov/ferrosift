//! Compiling a pipeline once and running it repeatedly.

use ferrosift::prelude::*;
use ferrosift::{Engine, ExecutionBudget, default_budget};

fn engine() -> Engine {
    Engine::portable().expect("registry validates")
}

#[test]
fn an_engine_exposes_the_whole_portable_catalog() {
    let engine = engine();
    assert!(engine.len() >= 68);
    assert!(!engine.is_empty());
    assert!(
        engine
            .registry()
            .catalog()
            .any(|spec| spec.id.as_str() == "encoding.base64.decode@1")
    );
}

#[test]
fn one_compile_serves_many_runs() {
    let engine = engine();
    let pipeline = engine
        .pipeline()
        .from_base64()
        .compile(&engine)
        .expect("compiles");

    for (encoded, expected) in [
        (&b"Zm9v"[..], &b"foo"[..]),
        (b"YmFy", b"bar"),
        (b"YmF6", b"baz"),
    ] {
        assert_eq!(pipeline.run_bytes(encoded).expect("runs"), expected);
    }
}

#[test]
fn a_compiled_pipeline_matches_the_one_shot_path() {
    let engine = engine();
    let built = pipeline().from_base64().to_hex();
    let compiled = built.compile(&engine).expect("compiles");

    let direct = built.run_bytes(b"Zm9v").expect("one-shot runs");
    let repeated = compiled.run_bytes(b"Zm9v").expect("compiled runs");
    assert_eq!(direct, repeated);
    assert_eq!(direct, b"66 6f 6f");
}

#[test]
fn compiled_pipelines_adapt_input_without_the_registry() {
    let engine = engine();
    // `from_base64` wants text; the caller still hands over bytes, and the
    // accepted representation was captured at compile time.
    let compiled = engine
        .pipeline()
        .from_base64()
        .compile(&engine)
        .expect("compiles");
    assert_eq!(compiled.run_bytes(b"aGVsbG8=").expect("runs"), b"hello");

    // `to_hex` wants bytes; the caller hands over text.
    let compiled = engine
        .pipeline()
        .to_hex()
        .compile(&engine)
        .expect("compiles");
    assert_eq!(compiled.run_text("Hi").expect("runs"), "48 69");
}

#[test]
fn compiled_pipelines_reach_the_pattern_engine() {
    let engine = engine();
    let compiled = engine
        .pipeline()
        .from_base64()
        .compile(&engine)
        .expect("compiles");

    let nodes = compiled
        .run_pattern("be u16 magic @ 0x00;", b"Q0FGRQ==")
        .expect("transform then parse");
    assert_eq!(nodes[0].value, NodeValue::Unsigned(0x4341));

    let options = EvalOptions {
        max_nodes: 1,
        ..EvalOptions::default()
    };
    let error = compiled
        .run_pattern_with("u8 many[8] @ 0;", b"Q0FGRQ==", &options)
        .expect_err("the node budget applies");
    assert_eq!(error.code(), "pattern.eval.node_budget_exceeded");
}

#[test]
fn compiling_rejects_an_unknown_operation_before_any_run() {
    let engine = engine();
    let error = pipeline()
        .step("encoding.does_not_exist@1", Arguments::new())
        .compile(&engine)
        .err()
        .expect("compilation must fail");
    assert_eq!(error.code(), "ferrosift.operation.unknown");
}

#[test]
fn the_budget_is_captured_at_compile_time() {
    let engine = engine();
    let tight = ExecutionBudget {
        max_output_bytes: 2,
        ..default_budget()
    };
    let compiled = pipeline()
        .budget(tight)
        .to_hex()
        .compile(&engine)
        .expect("compiles");
    let error = compiled
        .run_bytes(b"far too long for two bytes")
        .expect_err("the output budget applies");
    assert_eq!(error.code(), "core.operation.output_limit_exceeded");
}

#[test]
fn a_compiled_pipeline_reports_its_resolved_steps() {
    let engine = engine();
    let compiled = engine
        .pipeline()
        .from_base64()
        .to_hex()
        .compile(&engine)
        .expect("compiles");
    assert_eq!(compiled.len(), 2);
    assert!(!compiled.is_empty());
    assert_eq!(compiled.prepared().len(), 2);

    let empty = engine.pipeline().compile(&engine).expect("compiles");
    assert!(empty.is_empty());
}

#[test]
fn an_engine_can_be_built_from_a_reduced_registry() {
    use ferrosift_core::OperationRegistry;

    let mut registry = OperationRegistry::new();
    registry
        .register(ferrosift_operations::FromBase64::new())
        .expect("registers");
    let engine = Engine::with_registry(registry);
    assert_eq!(engine.len(), 1);

    let compiled = engine
        .pipeline()
        .from_base64()
        .compile(&engine)
        .expect("compiles against the reduced catalog");
    assert_eq!(compiled.run_bytes(b"Zm9v").expect("runs"), b"foo");

    // Anything outside the reduced catalog is refused at compile time.
    let error = engine
        .pipeline()
        .to_hex()
        .compile(&engine)
        .err()
        .expect("must not resolve");
    assert_eq!(error.code(), "ferrosift.operation.unknown");
}
