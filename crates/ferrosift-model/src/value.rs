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
    /// A number that need not be whole.
    ///
    /// Separate from [`ValueKind::Integer`] because the reference has one
    /// numeric type and it is a float: an operation that can answer 0.5 is
    /// not an integer operation whose answer happened to round.
    Number,
    /// Markup, which loses its tags when it is read as anything else.
    ///
    /// Not a flavour of [`ValueKind::Text`]. The reference strips the tags and
    /// resolves the entities whenever a markup value is asked for as bytes or
    /// as a string, so an operation downstream of one sees different
    /// characters than the markup contains. Text that merely happens to
    /// contain angle brackets is not this kind and keeps them.
    Markup,
    /// A nested structured value.
    Structured,
    /// A collection of virtual files.
    Files,
}

impl ValueKind {
    /// Every kind, so a consumer cannot silently miss one that was added.
    ///
    /// Kept here rather than written out where it is needed: the type-flow
    /// check held its own array of seven, which was correct until it was not,
    /// and nothing would have reported the omission.
    pub const ALL: [Self; 9] = [
        Self::Empty,
        Self::Bytes,
        Self::Text,
        Self::Boolean,
        Self::Integer,
        Self::Number,
        Self::Markup,
        Self::Structured,
        Self::Files,
    ];

    /// Whether a value of this kind can be re-read as `other`.
    ///
    /// The single source of truth for what [`Value::reinterpret`] will do, so
    /// a check made before execution cannot promise a conversion that
    /// execution then declines to perform.
    #[must_use]
    pub fn converts_to(self, other: Self) -> bool {
        // A kind always reads as itself; the rest is the table above.
        self == other
            || matches!(
                (self, other),
                (Self::Markup | Self::Number, Self::Text | Self::Bytes)
                    | (Self::Integer, Self::Number)
                    | (Self::Text, Self::Bytes)
            )
    }
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
    /// A number that need not be whole.
    Number(NumberValue),
    /// Markup, whose tags are removed when it is read as text or bytes.
    Markup(String),
    /// A nested structured value independent of any concrete format parser.
    Structured(StructuredValue),
    /// A collection of in-memory virtual files.
    Files(Vec<VirtualFile>),
}

/// A number, held as the reference holds one.
///
/// Wrapped rather than stored bare so that [`Value`] keeps `Eq`. Equality is
/// on the bit pattern, which differs from float equality in two places and
/// deliberately: two `NaN`s are the same carried value, and a negative zero is
/// not the same carried value as a positive one. Equality here asks whether
/// two values are the same value, not whether two measurements agree.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NumberValue(f64);

impl NumberValue {
    /// Carries `value`.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// The number itself.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for NumberValue {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl PartialEq for NumberValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for NumberValue {}

impl core::hash::Hash for NumberValue {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
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
            Self::Number(_) => ValueKind::Number,
            Self::Markup(_) => ValueKind::Markup,
            Self::Structured(_) => ValueKind::Structured,
            Self::Files(_) => ValueKind::Files,
        }
    }

    /// Re-reads this value as `kind`, the way the reference re-reads a dish.
    ///
    /// This is the piece the model was missing. The reference does not pass a
    /// value between steps untouched: asking a dish for another type converts
    /// it, and two of those conversions lose information on purpose. Without
    /// this, an operation downstream of one that produced markup receives the
    /// markup, while the same recipe in the reference receives the text with
    /// the tags taken out.
    ///
    /// Returns `None` when no conversion is defined, which leaves the caller
    /// to report a type mismatch rather than inventing one.
    #[must_use]
    pub fn reinterpret(self, kind: ValueKind) -> Option<Self> {
        if self.kind() == kind {
            return Some(self);
        }
        match (self, kind) {
            // Markup read as anything else is stripped and unescaped, which is
            // what makes this conversion lossy and why markup is its own kind.
            (Self::Markup(markup), ValueKind::Text) => Some(Self::Text(TextValue {
                text: strip_markup(&markup),
                encoding: TextEncoding::Utf8,
            })),
            (Self::Markup(markup), ValueKind::Bytes) => {
                Some(Self::Bytes(strip_markup(&markup).into_bytes()))
            }
            // A number becomes the digits the reference would print, not the
            // digits this language would.
            (Self::Number(number), ValueKind::Text) => Some(Self::Text(TextValue {
                text: render_number(number.get()),
                encoding: TextEncoding::Utf8,
            })),
            (Self::Number(number), ValueKind::Bytes) => {
                Some(Self::Bytes(render_number(number.get()).into_bytes()))
            }
            (Self::Integer(number), ValueKind::Number) => {
                Some(Self::Number(NumberValue::new(integer_to_float(number))))
            }
            (Self::Text(text), ValueKind::Bytes) => Some(Self::Bytes(text.text.into_bytes())),
            _ => None,
        }
    }
}

/// Removes tags and resolves entities, as the reference's markup dish does.
///
/// Deliberately the reference's rule and not a correct HTML parse: a `<` that
/// opens nothing still begins a tag here, because that is what the reference
/// does and a recipe downstream of one depends on it.
fn strip_markup(markup: &str) -> String {
    let mut output = String::with_capacity(markup.len());
    let mut depth = 0_u32;
    for character in markup.chars() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => output.push(character),
            _ => {}
        }
    }
    unescape_entities(&output)
}

/// Resolves the entities the reference's own escaper produces.
fn unescape_entities(text: &str) -> String {
    const ENTITIES: [(&str, char); 7] = [
        ("&amp;", '&'),
        ("&lt;", '<'),
        ("&gt;", '>'),
        ("&quot;", '"'),
        ("&#x27;", '\''),
        ("&#x2F;", '/'),
        ("&#x60;", '`'),
    ];
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    'outer: while !rest.is_empty() {
        for (entity, character) in ENTITIES {
            if let Some(tail) = rest.strip_prefix(entity) {
                output.push(character);
                rest = tail;
                continue 'outer;
            }
        }
        let mut characters = rest.chars();
        if let Some(character) = characters.next() {
            output.push(character);
        }
        rest = characters.as_str();
    }
    output
}

/// The digits the reference prints for a number.
///
/// Shortest round-trip decimal, with exponential notation past 1e21 and below
/// 1e-6 and a sign on the exponent that is never omitted.
fn render_number(value: f64) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value.is_infinite() {
        return String::from(if value > 0.0 { "Infinity" } else { "-Infinity" });
    }
    if value == 0.0 {
        return String::from("0");
    }
    let magnitude = if value < 0.0 { -value } else { value };
    if !(1e-6..1e21).contains(&magnitude) {
        let rendered = alloc::format!("{value:e}");
        return match rendered.split_once('e') {
            Some((mantissa, exponent)) if !exponent.starts_with('-') => {
                alloc::format!("{mantissa}e+{exponent}")
            }
            _ => rendered,
        };
    }
    alloc::format!("{value}")
}

/// Widens an integer into the reference's one numeric type.
#[expect(
    clippy::cast_precision_loss,
    reason = "the reference holds every number as this type, so the narrowing is its own"
)]
const fn integer_to_float(value: i128) -> f64 {
    value as f64
}

impl Value {
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
            Self::Number => "number",
            Self::Markup => "markup",
            Self::Structured => "structured",
            Self::Files => "files",
        })
    }
}
