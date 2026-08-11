//! Conversion between positional JSON and typed named arguments.

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_model::{ArgumentKind, ArgumentSpec, ArgumentValue, Arguments};
use serde_json::Value as JsonValue;

use crate::{
    error::ExportError,
    json_writer::CappedJson,
    profile::{MAX_ARGUMENT_DEPTH, MAX_SAFE_INTEGER},
};

pub(crate) enum ArgumentIssue {
    Extra {
        position: usize,
    },
    Missing {
        name: String,
    },
    Type {
        name: String,
        expected: ArgumentKind,
    },
    NumberRange {
        name: String,
    },
    Depth {
        name: String,
    },
}

enum ValueIssue {
    Type,
    NumberRange,
    Depth,
}

pub(crate) fn import_arguments(
    values: &[JsonValue],
    specifications: &[ArgumentSpec],
) -> Result<Arguments, Vec<ArgumentIssue>> {
    let mut arguments = Arguments::new();
    let mut issues = Vec::new();

    for (position, specification) in specifications.iter().enumerate() {
        let Some(value) = values.get(position) else {
            if specification.default.is_none() && specification.required {
                issues.push(ArgumentIssue::Missing {
                    name: specification.name.clone(),
                });
            }
            continue;
        };

        match import_value(value, specification.kind) {
            Ok(value) => {
                arguments.insert(specification.name.clone(), value);
            }
            Err(ValueIssue::Type) => issues.push(ArgumentIssue::Type {
                name: specification.name.clone(),
                expected: specification.kind,
            }),
            Err(ValueIssue::NumberRange) => issues.push(ArgumentIssue::NumberRange {
                name: specification.name.clone(),
            }),
            Err(ValueIssue::Depth) => issues.push(ArgumentIssue::Depth {
                name: specification.name.clone(),
            }),
        }
    }

    for position in specifications.len()..values.len() {
        issues.push(ArgumentIssue::Extra { position });
    }

    if issues.is_empty() {
        Ok(arguments)
    } else {
        Err(issues)
    }
}

fn import_value(value: &JsonValue, expected: ArgumentKind) -> Result<ArgumentValue, ValueIssue> {
    if expected == ArgumentKind::Bytes {
        return import_bytes(value)
            .map(ArgumentValue::Bytes)
            .ok_or(ValueIssue::Type);
    }

    let converted = import_untyped(value, 0)?;
    if expected.matches(&converted) {
        Ok(converted)
    } else {
        Err(ValueIssue::Type)
    }
}

fn import_bytes(value: &JsonValue) -> Option<Vec<u8>> {
    value
        .as_array()?
        .iter()
        .map(|item| u8::try_from(item.as_u64()?).ok())
        .collect()
}

fn import_untyped(value: &JsonValue, depth: usize) -> Result<ArgumentValue, ValueIssue> {
    if depth > MAX_ARGUMENT_DEPTH {
        return Err(ValueIssue::Depth);
    }
    match value {
        JsonValue::Null => Err(ValueIssue::Type),
        JsonValue::Bool(value) => Ok(ArgumentValue::Boolean(*value)),
        JsonValue::Number(value) => {
            let integer = value
                .as_i64()
                .map(i128::from)
                .or_else(|| value.as_u64().map(i128::from))
                .ok_or(ValueIssue::Type)?;
            if (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&integer) {
                Ok(ArgumentValue::Integer(integer))
            } else {
                Err(ValueIssue::NumberRange)
            }
        }
        JsonValue::String(value) => Ok(ArgumentValue::Text(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(|value| import_untyped(value, depth + 1))
            .collect::<Result<Vec<_>, _>>()
            .map(ArgumentValue::List),
        JsonValue::Object(values) => values
            .iter()
            .map(|(name, value)| Ok((name.clone(), import_untyped(value, depth + 1)?)))
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(ArgumentValue::Map),
    }
}

pub(crate) fn write_arguments(
    writer: &mut CappedJson,
    arguments: &Arguments,
    specifications: &[ArgumentSpec],
) -> Result<(), ExportError> {
    if arguments
        .keys()
        .any(|name| !specifications.iter().any(|spec| spec.name == *name))
    {
        return Err(ExportError::UndeclaredArgument);
    }

    let last_needed = specifications.iter().rposition(|specification| {
        arguments.contains_key(&specification.name)
            || specification.required
            || specification.default.is_some()
    });
    writer.push_raw("[")?;
    let Some(last_needed) = last_needed else {
        return writer.push_raw("]");
    };

    for (position, specification) in specifications[..=last_needed].iter().enumerate() {
        if position > 0 {
            writer.push_raw(",")?;
        }
        let value = arguments
            .get(&specification.name)
            .or(specification.default.as_ref())
            .ok_or(ExportError::MissingArgument)?;
        if !specification.kind.matches(value) {
            return Err(ExportError::ArgumentValue);
        }
        write_value(writer, value, 0)?;
    }
    writer.push_raw("]")
}

fn write_value(
    writer: &mut CappedJson,
    value: &ArgumentValue,
    depth: usize,
) -> Result<(), ExportError> {
    if depth > MAX_ARGUMENT_DEPTH {
        return Err(ExportError::ArgumentValue);
    }
    match value {
        ArgumentValue::Boolean(true) => writer.push_raw("true"),
        ArgumentValue::Boolean(false) => writer.push_raw("false"),
        ArgumentValue::Integer(value) if (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(value) => {
            writer.push_raw(&value.to_string())
        }
        ArgumentValue::Integer(_) => Err(ExportError::ArgumentValue),
        ArgumentValue::Text(value) => writer.push_string(value),
        ArgumentValue::Bytes(values) => {
            writer.push_raw("[")?;
            for (position, value) in values.iter().enumerate() {
                if position > 0 {
                    writer.push_raw(",")?;
                }
                writer.push_raw(&value.to_string())?;
            }
            writer.push_raw("]")
        }
        ArgumentValue::List(values) => {
            writer.push_raw("[")?;
            for (position, value) in values.iter().enumerate() {
                if position > 0 {
                    writer.push_raw(",")?;
                }
                write_value(writer, value, depth + 1)?;
            }
            writer.push_raw("]")
        }
        ArgumentValue::Map(values) => {
            writer.push_raw("{")?;
            for (position, (name, value)) in values.iter().enumerate() {
                if position > 0 {
                    writer.push_raw(",")?;
                }
                writer.push_string(name)?;
                writer.push_raw(":")?;
                write_value(writer, value, depth + 1)?;
            }
            writer.push_raw("}")
        }
    }
}
