//! Representation-preserving values used by recipes and operations.

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::ValueError;

/// The representation carried by a [`Value`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    /// No value is present.
    Empty,
    /// An uninterpreted byte sequence.
    Bytes,
    /// Text with an explicit encoding label.
    Text,
    /// A boolean value.
    Boolean,
    /// A signed integer.
    Integer,
    /// A nested structured value.
    Structured,
    /// A collection of virtual files.
    Files,
}

/// A representation-preserving value passed between `FerroSift` operations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    /// No value is present.
    Empty,
    /// An uninterpreted byte sequence.
    Bytes(Vec<u8>),
    /// Text with explicit encoding metadata.
    Text(TextValue),
    /// A boolean value.
    Boolean(bool),
    /// A signed 128-bit integer.
    Integer(i128),
    /// A nested structured value independent of any concrete format parser.
    Structured(StructuredValue),
    /// A collection of in-memory virtual files.
    Files(Vec<VirtualFile>),
}

/// Text together with the encoding that gives its representation meaning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextValue {
    /// Decoded Unicode scalar values stored as a Rust string.
    pub text: String,
    /// Encoding associated with the original or intended representation.
    pub encoding: TextEncoding,
}

/// Encoding metadata associated with a [`TextValue`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "label", rename_all = "snake_case")]
pub enum TextEncoding {
    /// UTF-8 encoding.
    Utf8,
    /// Little-endian UTF-16 encoding.
    Utf16Le,
    /// Big-endian UTF-16 encoding.
    Utf16Be,
    /// A named encoding not covered by a built-in variant.
    Named(String),
}

/// A deterministic tree for parsed structured data.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum StructuredValue {
    /// A null value.
    Null,
    /// A boolean value.
    Boolean(bool),
    /// A signed 128-bit integer.
    Integer(i128),
    /// A text value.
    Text(String),
    /// An uninterpreted byte sequence.
    Bytes(Vec<u8>),
    /// An ordered list of structured values.
    List(Vec<Self>),
    /// A key-sorted map of structured values.
    Object(BTreeMap<String, Self>),
}

/// An in-memory file carried as a value without granting filesystem access.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VirtualFile {
    /// Logical file name.
    pub name: String,
    /// Optional media type such as `application/octet-stream`.
    pub media_type: Option<String>,
    /// Complete file contents.
    pub contents: Vec<u8>,
}

impl Value {
    /// Returns the representation carried by this value.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Empty => ValueKind::Empty,
            Self::Bytes(_) => ValueKind::Bytes,
            Self::Text(_) => ValueKind::Text,
            Self::Boolean(_) => ValueKind::Boolean,
            Self::Integer(_) => ValueKind::Integer,
            Self::Structured(_) => ValueKind::Structured,
            Self::Files(_) => ValueKind::Files,
        }
    }

    /// Borrows the byte sequence carried by this value without copying it.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::TypeMismatch`] when this value is not [`Value::Bytes`].
    pub fn as_bytes(&self) -> Result<&[u8], ValueError> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            other => Err(ValueError::TypeMismatch {
                expected: ValueKind::Bytes,
                actual: other.kind(),
            }),
        }
    }

    /// Takes ownership of the byte sequence carried by this value.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::TypeMismatch`] when this value is not [`Value::Bytes`].
    pub fn try_into_bytes(self) -> Result<Vec<u8>, ValueError> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            other => Err(ValueError::TypeMismatch {
                expected: ValueKind::Bytes,
                actual: other.kind(),
            }),
        }
    }
}

impl fmt::Display for ValueKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "empty",
            Self::Bytes => "bytes",
            Self::Text => "text",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Structured => "structured",
            Self::Files => "files",
        })
    }
}
