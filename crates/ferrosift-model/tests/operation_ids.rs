//! Validation tests for stable recipe identifiers.

use ferrosift_model::{OperationId, StepId};

#[test]
fn canonical_operation_id_preserves_its_wire_value() {
    let id = OperationId::new("encoding.base64.decode@1").expect("valid operation ID");

    assert_eq!(id.as_str(), "encoding.base64.decode@1");
    assert_eq!(id.to_string(), "encoding.base64.decode@1");
    assert_eq!(
        serde_json::to_string(&id).expect("operation ID should serialize"),
        r#""encoding.base64.decode@1""#
    );
}

#[test]
fn static_operation_id_uses_the_same_canonical_value() {
    assert_eq!(STATIC_OPERATION_ID.as_str(), "encoding.hex.encode@1");
}

#[test]
fn ambiguous_operation_ids_are_rejected_with_a_stable_code() {
    for invalid in [
        "",
        "Base64.decode@1",
        "base64..decode@1",
        "base64.decode",
        "base64.decode@",
        "base64.decode@01",
        "base64.decode@1.0",
    ] {
        let error = OperationId::new(invalid).expect_err("operation ID should be invalid");
        assert_eq!(error.code(), "model.operation_id.invalid", "{invalid}");
    }
}

#[test]
fn operation_id_deserialization_uses_the_same_validation() {
    let error = serde_json::from_str::<OperationId>(r#""Base64.decode@1""#)
        .expect_err("invalid operation ID should not deserialize");

    assert!(error.to_string().contains("model.operation_id.invalid"));
}

#[test]
fn canonical_step_id_preserves_its_wire_value() {
    let id = StepId::new("decode-1").expect("valid step ID");

    assert_eq!(id.as_str(), "decode-1");
    assert_eq!(id.to_string(), "decode-1");
    assert_eq!(
        serde_json::from_str::<StepId>(r#""decode-1""#).expect("step ID should deserialize"),
        id
    );
}

#[test]
fn ambiguous_step_ids_are_rejected_with_a_stable_code() {
    for invalid in ["", "Decode-1", "decode 1", "-decode", "decode.1"] {
        let error = StepId::new(invalid).expect_err("step ID should be invalid");
        assert_eq!(error.code(), "model.step_id.invalid", "{invalid}");
    }
}
const STATIC_OPERATION_ID: ferrosift_model::OperationId =
    ferrosift_model::OperationId::from_static("encoding.hex.encode@1");
