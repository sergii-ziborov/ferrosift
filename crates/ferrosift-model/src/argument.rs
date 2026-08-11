//! Typed, recursive values used as operation arguments.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::ModelError;

/// A deterministic map from operation argument names to typed values.
pub type Arguments = BTreeMap<String, ArgumentValue>;

/// A portable argument value with an explicit representation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ArgumentValue {
    /// A boolean argument.
    Boolean(bool),
    /// A signed 128-bit integer argument.
    Integer(i128),
    /// A Unicode text argument.
    Text(String),
    /// An uninterpreted byte-string argument.
    Bytes(Vec<u8>),
    /// An ordered list of argument values.
    List(Vec<Self>),
    /// A key-sorted map of argument values.
    Map(Arguments),
}

#[derive(Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum ArgumentValueWire {
    Boolean(bool),
    Integer(i128),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<ArgumentValue>),
    Map(Arguments),
}

impl From<ArgumentValueWire> for ArgumentValue {
    fn from(value: ArgumentValueWire) -> Self {
        match value {
            ArgumentValueWire::Boolean(value) => Self::Boolean(value),
            ArgumentValueWire::Integer(value) => Self::Integer(value),
            ArgumentValueWire::Text(value) => Self::Text(value),
            ArgumentValueWire::Bytes(value) => Self::Bytes(value),
            ArgumentValueWire::List(value) => Self::List(value),
            ArgumentValueWire::Map(value) => Self::Map(value),
        }
    }
}

impl<'de> Deserialize<'de> for ArgumentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ArgumentValueWire::deserialize(deserializer)
            .map(Into::into)
            .map_err(|error| {
                D::Error::custom(format_args!(
                    "{}: {error}",
                    ModelError::UnknownArgumentKind.code()
                ))
            })
    }
}
