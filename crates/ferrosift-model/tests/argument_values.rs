//! Wire-format tests for portable recipe arguments.

use ferrosift_model::{ArgumentValue, Arguments};

fn round_trip(value: &ArgumentValue) -> ArgumentValue {
    let json = serde_json::to_string(value).expect("argument should serialize");
    serde_json::from_str(&json).expect("argument should deserialize")
}

#[test]
fn scalar_argument_kinds_remain_distinct() {
    let values = [
        ArgumentValue::Boolean(true),
        ArgumentValue::Integer(-7),
        ArgumentValue::Text("hello".into()),
        ArgumentValue::Bytes(vec![0, 255]),
    ];

    for value in values {
        assert_eq!(round_trip(&value), value);
    }
}

#[test]
fn byte_arguments_have_an_explicit_tagged_wire_format() {
    let value = ArgumentValue::Bytes(vec![0, 255]);

    assert_eq!(
        serde_json::to_string(&value).expect("bytes should serialize"),
        r#"{"kind":"bytes","value":[0,255]}"#
    );
}

#[test]
fn nested_maps_and_lists_round_trip_deterministically() {
    let mut nested = Arguments::new();
    nested.insert(
        "nested".into(),
        ArgumentValue::List(vec![ArgumentValue::Boolean(true)]),
    );
    let value = ArgumentValue::Map(nested);
    let expected = r#"{"kind":"map","value":{"nested":{"kind":"list","value":[{"kind":"boolean","value":true}]}}}"#;

    assert_eq!(
        serde_json::to_string(&value).expect("nested argument should serialize"),
        expected
    );
    assert_eq!(
        serde_json::from_str::<ArgumentValue>(expected)
            .expect("nested argument should deserialize"),
        value
    );
}

#[test]
fn unknown_argument_kinds_fail_with_a_stable_code() {
    let error = serde_json::from_str::<ArgumentValue>(r#"{"kind":"duration","value":1}"#)
        .expect_err("unknown argument kind should fail");

    assert!(error.to_string().contains("model.argument.unknown_kind"));
}
