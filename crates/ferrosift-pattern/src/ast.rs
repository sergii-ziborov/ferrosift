use alloc::string::String;
use alloc::vec::Vec;

/// A parsed pattern source: every declaration in the order it was written.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Pattern {
    /// Declarations in source order.
    pub declarations: Vec<Declaration>,
}

impl Pattern {
    /// Finds a named type declaration, ignoring placements.
    #[must_use]
    pub fn type_named(&self, name: &str) -> Option<&Declaration> {
        self.declarations.iter().find(|declaration| {
            matches!(
                declaration,
                Declaration::Struct(value) if value.name == name)
                || matches!(declaration, Declaration::Enum(value) if value.name == name)
                || matches!(declaration, Declaration::Bitfield(value) if value.name == name)
                || matches!(declaration, Declaration::Alias(value) if value.name == name)
        })
    }
}

/// A top-level declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Declaration {
    /// `struct Name { ... }`
    Struct(StructDeclaration),
    /// `enum Name : Type { ... }`
    Enum(EnumDeclaration),
    /// `bitfield Name { ... }`
    Bitfield(BitfieldDeclaration),
    /// `using Alias = Type;`
    Alias(AliasDeclaration),
    /// `Type name @ address;`
    Placement(Placement),
}

/// A composite type whose fields are laid out back to back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructDeclaration {
    /// Type name.
    pub name: String,
    /// Fields in layout order.
    pub fields: Vec<Field>,
}

/// One member of a struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Member name.
    pub name: String,
    /// Member type.
    pub type_reference: TypeReference,
    /// `Some(n)` when the member is a fixed-size array of `n` elements.
    pub array_length: Option<u128>,
}

/// A named set of integer constants over an explicit backing type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDeclaration {
    /// Type name.
    pub name: String,
    /// Backing integer type.
    pub backing: TypeReference,
    /// Constants in source order.
    pub entries: Vec<EnumEntry>,
}

/// One `Name` or `Name = value` entry of an enum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumEntry {
    /// Constant name.
    pub name: String,
    /// Resolved constant value; implicit entries continue the sequence.
    pub value: u128,
}

/// A packed set of bit-width members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitfieldDeclaration {
    /// Type name.
    pub name: String,
    /// Members in declaration order, most significant first.
    pub members: Vec<BitfieldMember>,
}

/// One `name : bits` member of a bitfield.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitfieldMember {
    /// Member name.
    pub name: String,
    /// Width in bits.
    pub bits: u32,
}

/// `using Alias = Type;`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasDeclaration {
    /// New name.
    pub name: String,
    /// Type the name refers to.
    pub target: TypeReference,
}

/// A variable placed at an absolute offset in the data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Variable name.
    pub name: String,
    /// Variable type.
    pub type_reference: TypeReference,
    /// `Some(n)` when the variable is a fixed-size array of `n` elements.
    pub array_length: Option<u128>,
    /// Absolute byte offset the variable starts at.
    pub address: u128,
}

/// A type together with any explicit endianness prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeReference {
    /// Which type is referenced.
    pub kind: TypeKind,
    /// Endianness forced by a `be` / `le` prefix, if any.
    pub endian: Option<Endian>,
}

/// Either a language built-in or a name declared elsewhere in the pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// A built-in scalar.
    Builtin(Builtin),
    /// A user-declared type, resolved during evaluation.
    Named(String),
}

/// Byte order for multi-byte scalars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    /// Most significant byte first.
    Big,
    /// Least significant byte first.
    Little,
}

/// The built-in scalar types of the supported subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    /// Unsigned integer of the given byte width.
    Unsigned(u32),
    /// Signed two's-complement integer of the given byte width.
    Signed(u32),
    /// IEEE-754 binary32.
    Float,
    /// IEEE-754 binary64.
    Double,
    /// One byte read as true / false.
    Bool,
    /// One byte read as a character.
    Char,
    /// Two bytes read as a UTF-16 code unit.
    Char16,
}

impl Builtin {
    /// Resolves a built-in type name, or `None` when the word is a user type.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "u8" => Self::Unsigned(1),
            "u16" => Self::Unsigned(2),
            "u24" => Self::Unsigned(3),
            "u32" => Self::Unsigned(4),
            "u48" => Self::Unsigned(6),
            "u64" => Self::Unsigned(8),
            "u96" => Self::Unsigned(12),
            "u128" => Self::Unsigned(16),
            "s8" => Self::Signed(1),
            "s16" => Self::Signed(2),
            "s24" => Self::Signed(3),
            "s32" => Self::Signed(4),
            "s48" => Self::Signed(6),
            "s64" => Self::Signed(8),
            "s96" => Self::Signed(12),
            "s128" => Self::Signed(16),
            "float" => Self::Float,
            "double" => Self::Double,
            "bool" => Self::Bool,
            "char" => Self::Char,
            "char16" => Self::Char16,
            _ => return None,
        })
    }

    /// Width in bytes.
    #[must_use]
    pub const fn size(self) -> u32 {
        match self {
            Self::Unsigned(size) | Self::Signed(size) => size,
            Self::Float => 4,
            Self::Double => 8,
            Self::Bool | Self::Char => 1,
            Self::Char16 => 2,
        }
    }
}
