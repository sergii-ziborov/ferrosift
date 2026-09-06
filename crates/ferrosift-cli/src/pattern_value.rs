//! JSON projections of pattern evaluation trees.

use ferrosift_pattern::{Endian, Node, NodeValue, ScalarArray};
use serde::Serialize;

/// One evaluated pattern field with absolute byte coordinates.
#[derive(Debug, Serialize)]
pub struct PatternNodeReport {
    /// Field, member, or variable name.
    pub name: String,
    /// Type as written in the pattern.
    pub type_name: String,
    /// Absolute byte offset in the evaluated artifact.
    pub offset: u64,
    /// Byte length occupied by the node.
    pub size: u64,
    /// Decoded value.
    pub value: PatternValueReport,
}

/// Decoded pattern content with JSON-safe integer tagging.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PatternValueReport {
    /// An unsigned integer, carried as a decimal string.
    Unsigned(String),
    /// A signed integer, carried as a decimal string.
    Signed(String),
    /// An IEEE-754 binary32 value.
    Float(f32),
    /// An IEEE-754 binary64 value.
    Double(f64),
    /// A boolean value.
    Bool(bool),
    /// A character.
    Char(char),
    /// An enum value.
    Enumerator {
        /// Matching constant name when one exists.
        name: Option<String>,
        /// Raw backing value as a decimal string.
        value: String,
    },
    /// Nested composite members.
    Group(Vec<PatternNodeReport>),
    /// Compact scalar array retained as bytes.
    Scalars(ScalarArrayReport),
}

/// Compact scalar-array projection that does not expand every element.
#[derive(Debug, Serialize)]
pub struct ScalarArrayReport {
    /// Element type spelling.
    pub element_type: &'static str,
    /// Byte order used to decode elements.
    pub endian: &'static str,
    /// Number of logical elements.
    pub length: usize,
    /// Raw backing bytes in layout order.
    pub bytes: Vec<u8>,
}

impl PatternNodeReport {
    /// Projects an evaluated node tree into a JSON-safe report.
    #[must_use]
    pub fn from_nodes(nodes: &[Node]) -> Vec<Self> {
        nodes.iter().map(Self::from).collect()
    }
}

impl From<&Node> for PatternNodeReport {
    fn from(node: &Node) -> Self {
        Self {
            name: node.name.clone(),
            type_name: node.type_name.clone(),
            offset: node.offset,
            size: node.size,
            value: PatternValueReport::from(&node.value),
        }
    }
}

impl From<&NodeValue> for PatternValueReport {
    fn from(value: &NodeValue) -> Self {
        match value {
            NodeValue::Unsigned(value) => Self::Unsigned(value.to_string()),
            NodeValue::Signed(value) => Self::Signed(value.to_string()),
            NodeValue::Float(value) => Self::Float(*value),
            NodeValue::Double(value) => Self::Double(*value),
            NodeValue::Bool(value) => Self::Bool(*value),
            NodeValue::Char(value) => Self::Char(*value),
            NodeValue::Enumerator { name, value } => Self::Enumerator {
                name: name.clone(),
                value: value.to_string(),
            },
            NodeValue::Group(nodes) => Self::Group(PatternNodeReport::from_nodes(nodes)),
            NodeValue::Scalars(array) => Self::Scalars(ScalarArrayReport::from(array)),
        }
    }
}

impl From<&ScalarArray> for ScalarArrayReport {
    fn from(array: &ScalarArray) -> Self {
        Self {
            element_type: array.element_type().name(),
            endian: match array.endian() {
                Endian::Big => "big",
                Endian::Little => "little",
            },
            length: array.len(),
            bytes: array.bytes().to_vec(),
        }
    }
}
