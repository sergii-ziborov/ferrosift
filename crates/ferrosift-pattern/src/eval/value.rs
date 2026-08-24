use alloc::string::String;
use alloc::vec::Vec;

/// One evaluated element of the data, with the exact bytes it came from.
///
/// Offsets and sizes are absolute byte positions in the evaluated buffer, so
/// a node can be highlighted in a hex view without re-deriving the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Field, member, or variable name; array elements carry `name[index]`.
    pub name: String,
    /// Type as written in the pattern, for display.
    pub type_name: String,
    /// Absolute byte offset the node starts at.
    pub offset: u64,
    /// Byte length the node occupies.
    pub size: u64,
    /// The decoded value.
    pub value: NodeValue,
}

/// The decoded content of a [`Node`].
#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    /// An unsigned integer.
    Unsigned(u128),
    /// A signed two's-complement integer.
    Signed(i128),
    /// An IEEE-754 binary32 value.
    Float(f32),
    /// An IEEE-754 binary64 value.
    Double(f64),
    /// A byte read as true / false.
    Bool(bool),
    /// A character.
    Char(char),
    /// An enum value, with the matching constant name when one exists.
    Enumerator {
        /// Name of the matching constant, if the value is named.
        name: Option<String>,
        /// The raw backing value.
        value: u128,
    },
    /// A struct, bitfield, or array with its children in layout order.
    Group(Vec<Node>),
}

impl Node {
    /// The byte offset just past this node.
    #[must_use]
    pub const fn end(&self) -> u64 {
        self.offset.saturating_add(self.size)
    }

    /// Children of a group node, or an empty slice for scalars.
    #[must_use]
    pub fn children(&self) -> &[Node] {
        match &self.value {
            NodeValue::Group(children) => children,
            _ => &[],
        }
    }

    /// Finds a direct child by name.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&Self> {
        self.children().iter().find(|child| child.name == name)
    }
}
