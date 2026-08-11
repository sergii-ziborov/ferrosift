//! Input/output constraints and typed argument declarations.

use alloc::{collections::BTreeSet, string::String};

use serde::{Deserialize, Serialize};

use crate::{ArgumentValue, ValueKind};

/// A constraint on an operation input or output representation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ValueConstraint {
    /// Every value representation is accepted.
    Any,
    /// Exactly one representation is accepted.
    Exact(ValueKind),
    /// Any representation in a deterministic non-empty set is accepted.
    OneOf(BTreeSet<ValueKind>),
}

impl ValueConstraint {
    /// Returns whether this constraint accepts the supplied value representation.
    #[must_use]
    pub fn accepts(&self, kind: ValueKind) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => *expected == kind,
            Self::OneOf(expected) => expected.contains(&kind),
        }
    }
}

/// The representation required by an operation argument.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentKind {
    /// A boolean argument.
    Boolean,
    /// A signed integer argument.
    Integer,
    /// A Unicode text argument.
    Text,
    /// An uninterpreted byte-string argument.
    Bytes,
    /// A recursively typed list argument.
    List,
    /// A recursively typed map argument.
    Map,
}

impl ArgumentKind {
    /// Returns whether `value` carries this exact representation.
    #[must_use]
    pub const fn matches(self, value: &ArgumentValue) -> bool {
        matches!(
            (self, value),
            (Self::Boolean, ArgumentValue::Boolean(_))
                | (Self::Integer, ArgumentValue::Integer(_))
                | (Self::Text, ArgumentValue::Text(_))
                | (Self::Bytes, ArgumentValue::Bytes(_))
                | (Self::List, ArgumentValue::List(_))
                | (Self::Map, ArgumentValue::Map(_))
        )
    }
}

/// One named and typed operation argument.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArgumentSpec {
    /// Stable argument name used in recipes.
    pub name: String,
    /// Human-facing explanation.
    pub description: String,
    /// Whether callers must supply the argument.
    pub required: bool,
    /// Required representation.
    pub kind: ArgumentKind,
    /// Optional representation-preserving default.
    pub default: Option<ArgumentValue>,
}
