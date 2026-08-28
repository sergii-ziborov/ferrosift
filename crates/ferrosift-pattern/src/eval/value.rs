use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{Builtin, Endian};

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
    /// A struct, bitfield, or array of composite elements, in layout order.
    Group(Vec<Node>),
    /// An array of a fixed-width scalar, kept as the bytes it was read from.
    ///
    /// One [`Node`] per element is the obvious representation and the wrong
    /// one. A `Node` carries two `String`s, so `u8 data[0x100000]` is a
    /// megabyte of subject and something over a hundred megabytes of tree,
    /// nearly all of it the words `data[524288]` and `u8` written out a
    /// million times. The bytes *are* the value; the elements are a view of
    /// them, and [`ScalarArray::get`] is that view.
    Scalars(ScalarArray),
}

/// The bytes of a scalar array, decoded one element at a time.
///
/// Only for arrays whose element type resolves to a built-in scalar, through
/// any number of `using` aliases. An array of structs, bitfields or enums
/// keeps a [`NodeValue::Group`], because those elements are trees rather than
/// numbers and there is nothing to defer.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarArray {
    element: Builtin,
    endian: Endian,
    bytes: Vec<u8>,
}

impl ScalarArray {
    pub(super) const fn new(element: Builtin, endian: Endian, bytes: Vec<u8>) -> Self {
        Self {
            element,
            endian,
            bytes,
        }
    }

    /// One element's width in bytes.
    ///
    /// Never zero for any built-in the language has; floored at one anyway, so
    /// that the length and the iterator cannot disagree about an element type
    /// that occupies nothing.
    const fn width(&self) -> usize {
        let width = self.element.size() as usize;
        if width == 0 { 1 } else { width }
    }

    /// How many elements the array holds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len() / self.width()
    }

    /// Whether the array holds no elements.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The element type, as written.
    #[must_use]
    pub const fn element_type(&self) -> Builtin {
        self.element
    }

    /// The byte order every element was read in.
    #[must_use]
    pub const fn endian(&self) -> Endian {
        self.endian
    }

    /// The raw bytes, in layout order.
    ///
    /// The whole point of the representation: a caller that wants the bytes —
    /// to hash them, write them out, or hand them to a decoder — takes them
    /// without decoding a single element.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Decodes one element, or `None` past the end.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<NodeValue> {
        let width = self.width();
        let start = index.checked_mul(width)?;
        let slice = self.bytes.get(start..start.checked_add(width)?)?;
        Some(decode(self.element, self.endian, slice))
    }

    /// Decodes every element, in order.
    ///
    /// Over the byte chunks rather than over the indices, so the iterator is
    /// total by construction and knows its own length.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = NodeValue> + '_ {
        self.bytes
            .chunks_exact(self.width())
            .map(|chunk| decode(self.element, self.endian, chunk))
    }
}

/// Reads one scalar out of exactly its own bytes.
fn decode(element: Builtin, endian: Endian, bytes: &[u8]) -> NodeValue {
    let mut raw: u128 = 0;
    match endian {
        Endian::Big => {
            for byte in bytes {
                raw = (raw << 8) | u128::from(*byte);
            }
        }
        Endian::Little => {
            for byte in bytes.iter().rev() {
                raw = (raw << 8) | u128::from(*byte);
            }
        }
    }
    scalar_value(element, raw)
}

/// Interprets a scalar's raw bits as the value its type names.
///
/// Shared by the two paths that produce one: a lone field, which reads its
/// bytes through the source, and an element of a [`ScalarArray`], which reads
/// them out of the block already held. Writing it once is what keeps the two
/// from drifting — an array element that decoded differently from the same
/// type written on its own would be the hardest kind of difference to notice.
pub(super) fn scalar_value(element: Builtin, raw: u128) -> NodeValue {
    match element {
        Builtin::Unsigned(_) => NodeValue::Unsigned(raw),
        Builtin::Signed(size) => NodeValue::Signed(sign_extend(raw, size)),
        Builtin::Float => NodeValue::Float(f32::from_bits(truncate_u32(raw))),
        Builtin::Double => NodeValue::Double(f64::from_bits(truncate_u64(raw))),
        Builtin::Bool => NodeValue::Bool(raw != 0),
        Builtin::Char => NodeValue::Char(char::from(truncate_u8(raw))),
        Builtin::Char16 => NodeValue::Char(
            char::from_u32(u32::from(truncate_u16(raw))).unwrap_or(char::REPLACEMENT_CHARACTER),
        ),
    }
}

/// Widens an `n`-byte two's-complement value to `i128`.
fn sign_extend(raw: u128, size: u32) -> i128 {
    let bits = size.saturating_mul(8);
    if bits == 0 || bits >= 128 {
        return raw.cast_signed();
    }
    let sign_bit = 1_u128 << (bits - 1);
    if raw & sign_bit == 0 {
        raw.cast_signed()
    } else {
        raw.cast_signed().wrapping_sub(1_i128 << bits)
    }
}

fn truncate_u8(value: u128) -> u8 {
    u8::try_from(value & 0xff).unwrap_or(0)
}

fn truncate_u16(value: u128) -> u16 {
    u16::try_from(value & 0xffff).unwrap_or(0)
}

fn truncate_u32(value: u128) -> u32 {
    u32::try_from(value & 0xffff_ffff).unwrap_or(0)
}

fn truncate_u64(value: u128) -> u64 {
    u64::try_from(value & u128::from(u64::MAX)).unwrap_or(0)
}

impl Node {
    /// The byte offset just past this node.
    #[must_use]
    pub const fn end(&self) -> u64 {
        self.offset.saturating_add(self.size)
    }

    /// Children of a group node, or an empty slice for anything else.
    ///
    /// An array of a built-in scalar has **no** children: its elements are
    /// bytes rather than nodes, and [`Node::element`] reads them. See
    /// [`NodeValue::Scalars`] for why.
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

    /// How many elements an array holds, whichever way it is stored.
    ///
    /// Zero for anything that is not an array — a scalar has no elements, and
    /// a struct has members rather than elements.
    #[must_use]
    pub fn element_count(&self) -> usize {
        match &self.value {
            NodeValue::Group(children) => children.len(),
            NodeValue::Scalars(array) => array.len(),
            _ => 0,
        }
    }

    /// One element of an array, whichever way it is stored.
    ///
    /// The two representations are an implementation detail of *size*, not of
    /// meaning, so this reads either — which is what lets a caller walk an
    /// array without first asking how it happens to be held.
    #[must_use]
    pub fn element(&self, index: usize) -> Option<NodeValue> {
        match &self.value {
            NodeValue::Group(children) => children.get(index).map(|child| child.value.clone()),
            NodeValue::Scalars(array) => array.get(index),
            _ => None,
        }
    }
}
