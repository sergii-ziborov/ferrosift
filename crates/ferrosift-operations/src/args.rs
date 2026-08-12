use alloc::{collections::BTreeMap, string::String};

use ferrosift_core::OperationError;
use ferrosift_model::{ArgumentKind, ArgumentSpec, ArgumentValue, Arguments};

pub(crate) fn text_argument(name: &str, description: &str, default: &str) -> ArgumentSpec {
    ArgumentSpec {
        name: String::from(name),
        description: String::from(description),
        required: false,
        kind: ArgumentKind::Text,
        default: Some(ArgumentValue::Text(String::from(default))),
    }
}

pub(crate) fn integer_argument(name: &str, description: &str, default: i128) -> ArgumentSpec {
    ArgumentSpec {
        name: String::from(name),
        description: String::from(description),
        required: false,
        kind: ArgumentKind::Integer,
        default: Some(ArgumentValue::Integer(default)),
    }
}

pub(crate) fn boolean_argument(name: &str, description: &str, default: bool) -> ArgumentSpec {
    ArgumentSpec {
        name: String::from(name),
        description: String::from(description),
        required: false,
        kind: ArgumentKind::Boolean,
        default: Some(ArgumentValue::Boolean(default)),
    }
}

pub(crate) fn text_value<'a>(
    arguments: &'a Arguments,
    name: &str,
) -> Result<&'a str, OperationError> {
    match arguments.get(name) {
        Some(ArgumentValue::Text(value)) => Ok(value),
        _ => Err(OperationError::InvalidArguments),
    }
}

pub(crate) fn integer_value(arguments: &Arguments, name: &str) -> Result<i128, OperationError> {
    match arguments.get(name) {
        Some(ArgumentValue::Integer(value)) => Ok(*value),
        _ => Err(OperationError::InvalidArguments),
    }
}

pub(crate) fn boolean_value(arguments: &Arguments, name: &str) -> Result<bool, OperationError> {
    match arguments.get(name) {
        Some(ArgumentValue::Boolean(value)) => Ok(*value),
        _ => Err(OperationError::InvalidArguments),
    }
}

pub(crate) fn map_argument(
    name: &str,
    description: &str,
    default: BTreeMap<String, ArgumentValue>,
) -> ArgumentSpec {
    ArgumentSpec {
        name: String::from(name),
        description: String::from(description),
        required: false,
        kind: ArgumentKind::Map,
        default: Some(ArgumentValue::Map(default)),
    }
}

pub(crate) fn map_value<'a>(
    arguments: &'a Arguments,
    name: &str,
) -> Result<&'a Arguments, OperationError> {
    match arguments.get(name) {
        Some(ArgumentValue::Map(value)) => Ok(value),
        _ => Err(OperationError::InvalidArguments),
    }
}

pub(crate) fn toggle_string_default(option: &str, string: &str) -> BTreeMap<String, ArgumentValue> {
    BTreeMap::from([
        (
            String::from("option"),
            ArgumentValue::Text(String::from(option)),
        ),
        (
            String::from("string"),
            ArgumentValue::Text(String::from(string)),
        ),
    ])
}

pub(crate) fn toggle_string_parts(value: &Arguments) -> Result<(&str, &str), OperationError> {
    let option = match value.get("option") {
        Some(ArgumentValue::Text(value)) => value.as_str(),
        _ => return Err(OperationError::InvalidArguments),
    };
    let string = match value.get("string") {
        Some(ArgumentValue::Text(value)) => value.as_str(),
        _ => return Err(OperationError::InvalidArguments),
    };
    Ok((option, string))
}
