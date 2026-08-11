//! Explicit conversion tests for `FerroSift` values.

use ferrosift_model::{TextEncoding, TextValue, Value, ValueError, ValueKind};

#[test]
fn bytes_can_be_borrowed_without_copying() {
    let value = Value::Bytes(vec![0x00, 0xff]);
    assert_eq!(value.as_bytes(), Ok(&[0x00, 0xff][..]));
}

#[test]
fn bytes_can_be_taken_explicitly() {
    assert_eq!(Value::Bytes(vec![1, 2]).try_into_bytes(), Ok(vec![1, 2]));
}

#[test]
fn text_is_not_implicitly_converted_to_bytes() {
    let value = Value::Text(TextValue {
        text: "hello".into(),
        encoding: TextEncoding::Utf8,
    });
    let expected = ValueError::TypeMismatch {
        expected: ValueKind::Bytes,
        actual: ValueKind::Text,
    };

    assert_eq!(value.as_bytes(), Err(expected));
    assert_eq!(value.try_into_bytes(), Err(expected));
    assert_eq!(expected.to_string(), "expected bytes, found text");
}
