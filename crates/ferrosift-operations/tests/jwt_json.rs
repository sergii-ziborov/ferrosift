//! Curated vectors for the JWT / JSON pilot cluster (FS-A13).

use ferrosift_core::{ExecutionBudget, NeverCancelled, Operation, OperationContext};
use ferrosift_model::{Arguments, CapabilitySet, StructuredValue, TextEncoding, TextValue, Value};
use ferrosift_operations::{JsonMinify, JwtDecode};

fn context() -> OperationContext<'static> {
    OperationContext::new(
        ExecutionBudget::generous(),
        &NeverCancelled,
        CapabilitySet::new(),
    )
}

fn text(input: &str) -> Value {
    Value::Text(TextValue {
        text: input.to_owned(),
        encoding: TextEncoding::Utf8,
    })
}

#[test]
fn jwt_decode_returns_payload_only_without_verifying() {
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let mut ctx = context();
    let value = JwtDecode::new()
        .execute(text(token), &Arguments::new(), &mut ctx)
        .expect("decode");
    let Value::Structured(StructuredValue::Object(entries)) = value else {
        panic!("expected structured object");
    };
    assert_eq!(
        entries,
        vec![
            ("sub".into(), StructuredValue::Text("1234567890".into())),
            ("name".into(), StructuredValue::Text("John Doe".into())),
            ("iat".into(), StructuredValue::Integer(1_516_239_022)),
        ]
    );
}

#[test]
fn jwt_decode_refuses_a_truncated_token() {
    let mut ctx = context();
    let error = JwtDecode::new()
        .execute(
            text("eyJhbGciOiJIUzI1NiJ9.e30"),
            &Arguments::new(),
            &mut ctx,
        )
        .expect_err("truncated");
    assert!(matches!(
        error,
        ferrosift_core::OperationError::Failed { .. }
    ));
}

#[test]
fn json_minify_strips_whitespace_outside_strings() {
    let mut ctx = context();
    let value = JsonMinify::new()
        .execute(
            text("{\n  \"a\": 1,\n  \"b\": \"keep  spaces\"\n}"),
            &Arguments::new(),
            &mut ctx,
        )
        .expect("minify");
    let Value::Text(text) = value else {
        panic!("expected text");
    };
    assert_eq!(text.text, r#"{"a":1,"b":"keep  spaces"}"#);
}

#[test]
fn aliases_are_registered() {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let names: Vec<&str> = registry
        .catalog()
        .flat_map(|spec| spec.aliases.iter().map(|alias| alias.name.as_str()))
        .collect();
    assert!(names.contains(&"JWT Decode"));
    assert!(names.contains(&"JSON Minify"));
}
