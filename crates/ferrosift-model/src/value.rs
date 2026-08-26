//! Representation-preserving values used by recipes and operations.

use alloc::{string::String, vec::Vec};
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{DecimalValue, ValueError};

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
    /// A decimal of unbounded size and precision.
    ///
    /// Separate from [`ValueKind::Number`] because the reference has both and
    /// they disagree: a float loses digits above 2^53 and a decimal does not,
    /// so an operation that answers with one is not answering with the other.
    Decimal,
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
    pub const ALL: [Self; 10] = [
        Self::Empty,
        Self::Bytes,
        Self::Text,
        Self::Boolean,
        Self::Integer,
        Self::Number,
        Self::Decimal,
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
        // A kind always reads as itself. Otherwise the question is only
        // whether one end can be written as canonical bytes and the other read
        // from them -- two predicates rather than a hundred ordered pairs, and
        // therefore incapable of disagreeing with `Value::reinterpret`.
        self == other || (self.has_dish_bytes() && other.reads_dish_bytes())
    }

    /// Whether a value of this kind can be written as canonical bytes.
    const fn has_dish_bytes(self) -> bool {
        !matches!(self, Self::Empty | Self::Boolean | Self::Files)
    }

    /// Whether a value of this kind can be read back from canonical bytes.
    ///
    /// `Integer` and `Structured` are writable but not readable. An integer
    /// read from digits would have to decide what to do with a fraction the
    /// reference would have kept, and a structure would need a JSON parser
    /// obliged to agree with `JSON.parse`. Both are absent rather than
    /// approximated.
    const fn reads_dish_bytes(self) -> bool {
        matches!(
            self,
            Self::Bytes | Self::Text | Self::Markup | Self::Number | Self::Decimal
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
    /// A decimal of unbounded size and precision.
    Decimal(DecimalValue),
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
    /// Named members, in the order they were added.
    ///
    /// Insertion order rather than a sorted map, because the reference
    /// enumerates an object's properties in a specific order and sorting is
    /// not it: integer-like keys come first in numeric order, and every other
    /// key follows in the order it was added. A sorted map cannot express
    /// that, and a rendering built from one puts `10` before `2` and `a`
    /// before `b` -- neither of which is what the reference writes.
    Object(Vec<(String, Self)>),
}

impl StructuredValue {
    /// The keys of an object in the order the reference enumerates them.
    ///
    /// Integer-like keys first, ascending, then the rest in insertion order.
    /// "Integer-like" is narrower than "parses as a number": a leading zero,
    /// a sign, or a fraction disqualifies a key, so `01` and `-1` are ordinary
    /// keys while `1` is an index. That is checked against the real engine in
    /// `tests/dish.rs` rather than taken from the specification's wording.
    #[must_use]
    pub fn enumeration_order(entries: &[(String, Self)]) -> Vec<usize> {
        let mut indexed: Vec<(u64, usize)> = Vec::new();
        let mut named: Vec<usize> = Vec::new();
        for (position, (key, _)) in entries.iter().enumerate() {
            match integer_index(key) {
                Some(index) => indexed.push((index, position)),
                None => named.push(position),
            }
        }
        indexed.sort_unstable();
        let mut order: Vec<usize> = indexed.into_iter().map(|(_, position)| position).collect();
        order.extend(named);
        order
    }
}

/// The number a key stands for, when the key is a canonical integer index.
///
/// Canonical means the key is exactly how the number prints: no sign, no
/// leading zero unless the key *is* `0`, no fraction. Anything else is an
/// ordinary key however numeric it looks.
fn integer_index(key: &str) -> Option<u64> {
    if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if key.len() > 1 && key.starts_with('0') {
        return None;
    }
    // The reference treats a canonical integer up to 2^53 - 1 as an index,
    // which is the largest whole number its one numeric type can hold exactly.
    key.parse::<u64>().ok().filter(|value| *value < (1 << 53))
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
            Self::Decimal(_) => ValueKind::Decimal,
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
        Self::from_dish_bytes(kind, self.into_dish_bytes()?)
    }

    /// The canonical bytes this value converts through.
    ///
    /// One arm per kind, mirroring the reference's dish types. `Boolean`,
    /// `Empty`, and `Files` have no counterpart there and so no byte form
    /// here: giving them one would invent a conversion the reference does not
    /// define, and would quietly accept a recipe it refuses.
    #[must_use]
    pub fn into_dish_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Text(text) => Some(text_to_bytes(&text.text)),
            Self::Integer(number) => Some(text_to_bytes(&render_number(integer_to_float(number)))),
            Self::Number(number) => Some(text_to_bytes(&render_number(number.get()))),
            Self::Decimal(decimal) => Some(text_to_bytes(&decimal.to_fixed())),
            // Markup loses its tags and entities here, which is the whole
            // reason it is a kind: a step after one sees this, not the markup.
            Self::Markup(markup) => Some(text_to_bytes(&strip_markup(&markup))),
            Self::Structured(value) => Some(text_to_bytes(&render_structured(&value, 0))),
            Self::Empty | Self::Boolean(_) | Self::Files(_) => None,
        }
    }

    /// Reads canonical bytes back as `kind`.
    ///
    /// `Structured` is deliberately absent. Reading it back means parsing
    /// JSON, and a parser obliged to agree with `JSON.parse` on every input is
    /// its own piece of work rather than a line here. A structure can be read
    /// *as* something else but not reconstructed from it, and the asymmetry is
    /// reported by [`ValueKind::converts_to`] so that preflight refuses such a
    /// step rather than execution failing on it.
    #[must_use]
    pub fn from_dish_bytes(kind: ValueKind, bytes: Vec<u8>) -> Option<Self> {
        match kind {
            ValueKind::Bytes => Some(Self::Bytes(bytes)),
            ValueKind::Text => Some(Self::Text(TextValue {
                text: bytes_to_text(&bytes),
                encoding: TextEncoding::Utf8,
            })),
            ValueKind::Markup => Some(Self::Markup(bytes_to_text(&bytes))),
            ValueKind::Decimal => Some(Self::Decimal(DecimalValue::parse(&bytes_to_text(&bytes)))),
            ValueKind::Number => Some(Self::Number(NumberValue::new(parse_number(
                &bytes_to_text(&bytes),
            )))),
            ValueKind::Empty
            | ValueKind::Boolean
            | ValueKind::Integer
            | ValueKind::Structured
            | ValueKind::Files => None,
        }
    }
}

/// Turns text into bytes the way the reference does, which is not UTF-8.
///
/// One byte per UTF-16 code unit while every unit fits in one, and the whole
/// string as UTF-8 the moment one does not. So `é` is the single byte `0xE9`
/// and not the pair `0xC3 0xA9` -- a difference this crate had wrong until the
/// conversion moved here, because nothing chained a step past text carrying a
/// character in that range.
fn text_to_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match u8::try_from(unit) {
            Ok(byte) => bytes.push(byte),
            Err(_) => return text.as_bytes().to_vec(),
        }
    }
    bytes
}

/// Reads bytes back as text, preferring UTF-8 and falling back a byte at a time.
///
/// The fallback is not a guess: the reference reads a buffer as UTF-8 and
/// keeps the byte values when that fails, so bytes that are not valid UTF-8
/// arrive as the characters of those byte values rather than as an error.
fn bytes_to_text(bytes: &[u8]) -> String {
    match core::str::from_utf8(bytes) {
        Ok(text) => String::from(text),
        Err(_) => bytes.iter().map(|byte| char::from(*byte)).collect(),
    }
}

/// Reads a number the way `parseFloat` does.
///
/// It reads the longest prefix that looks like a number and ignores the rest,
/// answering not-a-number only when the prefix is empty. `12abc` is twelve
/// there, and a parser that refused it would reject a conversion the reference
/// performs.
fn parse_number(text: &str) -> f64 {
    let trimmed = text.trim_start();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_point = false;
    let mut seen_exponent = false;
    for (index, character) in trimmed.char_indices() {
        let acceptable = match character {
            '0'..='9' => {
                seen_digit = true;
                true
            }
            '+' | '-' => {
                // A sign is only part of the number at the very start, or
                // immediately after the exponent marker.
                index == 0 || trimmed[..index].ends_with(['e', 'E'])
            }
            '.' => {
                !seen_point && !seen_exponent && {
                    seen_point = true;
                    true
                }
            }
            'e' | 'E' => {
                seen_digit && !seen_exponent && {
                    seen_exponent = true;
                    true
                }
            }
            _ => false,
        };
        if !acceptable {
            break;
        }
        end = index + character.len_utf8();
    }
    if !seen_digit {
        return f64::NAN;
    }
    trimmed[..end].parse::<f64>().unwrap_or(f64::NAN)
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
            Self::Decimal => "decimal",
            Self::Markup => "markup",
            Self::Structured => "structured",
            Self::Files => "files",
        })
    }
}

/// Renders a structured value as `JSON.stringify(value, null, 4)` writes it.
///
/// The four spaces are bytes rather than a display choice: the reference's
/// JSON dish converts with that indent, so a recipe reading the output of one
/// sees them.
///
/// **Key order is a known divergence.** `StructuredValue::Object` is a sorted
/// map and `JSON.stringify` writes keys in insertion order, so the two agree
/// only where a value's keys happen to sort into the order they were added.
/// They do for every operation that uses this today, and an operation whose
/// keys do not would need an order-preserving map before it could claim
/// compatibility. That is recorded in `docs/value-model.md` rather than left
/// to be discovered.
fn render_structured(value: &StructuredValue, indent: usize) -> String {
    let inner = indent + 4;
    match value {
        StructuredValue::Null => String::from("null"),
        StructuredValue::Boolean(flag) => String::from(if *flag { "true" } else { "false" }),
        StructuredValue::Integer(number) => render_number(integer_to_float(*number)),
        StructuredValue::Text(text) => render_json_string(text),
        // A byte run has no JSON spelling of its own, so it is written as the
        // numbers it holds -- the same shape a list of them would take.
        StructuredValue::Bytes(bytes) => render_list(
            &bytes
                .iter()
                .map(|byte| StructuredValue::Integer(i128::from(*byte)))
                .collect::<Vec<_>>(),
            indent,
            inner,
        ),
        StructuredValue::List(values) => render_list(values, indent, inner),
        StructuredValue::Object(entries) => {
            if entries.is_empty() {
                return String::from("{}");
            }
            let mut output = String::from("{\n");
            // Written in the order the reference enumerates them, which is
            // neither insertion order nor sorted order but a mixture of both.
            for (position, entry) in StructuredValue::enumeration_order(entries)
                .into_iter()
                .enumerate()
            {
                let (key, value) = &entries[entry];
                if position > 0 {
                    output.push_str(",\n");
                }
                for _ in 0..inner {
                    output.push(' ');
                }
                output.push_str(&render_json_string(key));
                output.push_str(": ");
                output.push_str(&render_structured(value, inner));
            }
            output.push('\n');
            for _ in 0..indent {
                output.push(' ');
            }
            output.push('}');
            output
        }
    }
}

/// A JSON array, or `[]` when it holds nothing.
fn render_list(values: &[StructuredValue], indent: usize, inner: usize) -> String {
    if values.is_empty() {
        return String::from("[]");
    }
    let mut output = String::from("[\n");
    for (position, value) in values.iter().enumerate() {
        if position > 0 {
            output.push_str(",\n");
        }
        for _ in 0..inner {
            output.push(' ');
        }
        output.push_str(&render_structured(value, inner));
    }
    output.push('\n');
    for _ in 0..indent {
        output.push(' ');
    }
    output.push(']');
    output
}

/// A JSON string, escaped the way `JSON.stringify` escapes one.
fn render_json_string(text: &str) -> String {
    let mut output = String::with_capacity(text.len() + 2);
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            // Everything below a space has no literal spelling and is written
            // as a four-digit escape, lower-case as the reference writes it.
            control if control < ' ' => {
                use core::fmt::Write as _;
                // Ignoring the result on purpose: writing into a `String`
                // cannot fail, and the only alternative is to allocate a
                // second one just to discard it.
                let _ = write!(output, "\\u{:04x}", u32::from(control));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}
