//! Deterministic logical payload measurement for portable values.

use ferrosift_model::{StructuredValue, Value, VirtualFile};

pub(crate) fn logical_size(value: &Value) -> u64 {
    match value {
        Value::Empty => 0,
        Value::Bytes(bytes) => size_of_len(bytes.len()),
        Value::Text(text) => size_of_len(text.text.len()),
        Value::Boolean(_) => 1,
        Value::Integer(_) => 16,
        Value::Structured(value) => structured_size(value),
        Value::Files(files) => saturated_sum(files.iter().map(file_size)),
    }
}

fn structured_size(value: &StructuredValue) -> u64 {
    match value {
        StructuredValue::Null => 0,
        StructuredValue::Boolean(_) => 1,
        StructuredValue::Integer(_) => 16,
        StructuredValue::Text(text) => size_of_len(text.len()),
        StructuredValue::Bytes(bytes) => size_of_len(bytes.len()),
        StructuredValue::List(values) => saturated_sum(values.iter().map(structured_size)),
        StructuredValue::Object(entries) => saturated_sum(
            entries
                .iter()
                .map(|(key, value)| size_of_len(key.len()).saturating_add(structured_size(value))),
        ),
    }
}

fn file_size(file: &VirtualFile) -> u64 {
    size_of_len(file.name.len())
        .saturating_add(
            file.media_type
                .as_ref()
                .map_or(0, |value| size_of_len(value.len())),
        )
        .saturating_add(size_of_len(file.contents.len()))
}

fn saturated_sum(values: impl Iterator<Item = u64>) -> u64 {
    values.fold(0, u64::saturating_add)
}

fn size_of_len(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}
