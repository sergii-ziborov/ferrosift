//! Round-trip tests for `FerroSift` values.

use std::collections::BTreeMap;

use ferrosift_model::{StructuredValue, TextEncoding, TextValue, Value, ValueKind, VirtualFile};

fn round_trip(value: &Value) -> Value {
    let json = serde_json::to_string(value).expect("value should serialize");
    serde_json::from_str(&json).expect("value should deserialize")
}

#[test]
fn empty_bytes_and_text_remain_distinct() {
    let values = [
        Value::Empty,
        Value::Bytes(Vec::new()),
        Value::Text(TextValue {
            text: String::new(),
            encoding: TextEncoding::Utf8,
        }),
    ];

    assert_ne!(values[0], values[1]);
    assert_ne!(values[1], values[2]);
    assert_eq!(
        values.map(|value| value.kind()),
        [ValueKind::Empty, ValueKind::Bytes, ValueKind::Text]
    );
}

#[test]
fn non_utf8_bytes_round_trip_without_loss() {
    let value = Value::Bytes(vec![0x00, 0x7f, 0x80, 0xff]);
    assert_eq!(round_trip(&value), value);
}

#[test]
fn byte_wire_format_is_explicitly_tagged() {
    let value = Value::Bytes(vec![0x00, 0xff]);
    let json = serde_json::to_string(&value).expect("value should serialize");
    assert_eq!(json, r#"{"kind":"bytes","value":[0,255]}"#);
}

#[test]
fn text_round_trip_preserves_named_encoding() {
    let value = Value::Text(TextValue {
        text: "Привет".into(),
        encoding: TextEncoding::Named("windows-1251".into()),
    });
    assert_eq!(round_trip(&value), value);
}

#[test]
fn nested_structured_value_round_trips_deterministically() {
    let mut object = BTreeMap::new();
    object.insert("enabled".into(), StructuredValue::Boolean(true));
    object.insert("count".into(), StructuredValue::Integer(-7));
    object.insert(
        "items".into(),
        StructuredValue::List(vec![
            StructuredValue::Null,
            StructuredValue::Text("x".into()),
        ]),
    );
    let value = Value::Structured(StructuredValue::Object(object));

    let first = serde_json::to_string(&value).expect("value should serialize");
    let second = serde_json::to_string(&round_trip(&value)).expect("value should reserialize");
    assert_eq!(second, first);
}

#[test]
fn virtual_files_preserve_name_media_type_and_contents() {
    let value = Value::Files(vec![VirtualFile {
        name: "sample.bin".into(),
        media_type: Some("application/octet-stream".into()),
        contents: vec![0x00, 0xff],
    }]);
    assert_eq!(round_trip(&value), value);
}
