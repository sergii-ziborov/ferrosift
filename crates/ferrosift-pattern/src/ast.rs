use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// A value computed from literals and fields already read.
///
/// Expressions are what let a pattern describe a format rather than one file:
/// an array whose length is a header field, a placement past a base address, a
/// member present only when a flag is set. Every position that took a literal
/// integer takes one of these instead, and a bare literal is still the common
/// case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expression {
    /// An integer literal.
    Integer(u128),
    /// `true` or `false`.
    Bool(bool),
    /// A character literal, held as its code point.
    Char(char),
    /// A dotted path to a field already read in the enclosing scope.
    Path(Vec<String>),
    /// `$`, the offset the field being evaluated starts at.
    Offset,
    /// `sizeof(...)`, in bytes.
    SizeOf(SizeOfTarget),
    /// A prefix operator applied to one operand.
    Unary {
        /// Which operator.
        operator: UnaryOperator,
        /// What it applies to.
        operand: Box<Expression>,
    },
    /// An infix operator applied to two operands.
    Binary {
        /// Which operator.
        operator: BinaryOperator,
        /// Left-hand operand.
        left: Box<Expression>,
        /// Right-hand operand.
        right: Box<Expression>,
    },
    /// `condition ? when_true : when_false`.
    Conditional {
        /// The test.
        condition: Box<Expression>,
        /// Value when the test holds.
        when_true: Box<Expression>,
        /// Value otherwise.
        when_false: Box<Expression>,
    },
}

/// What a `sizeof` is asking about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SizeOfTarget {
    /// A built-in type, whose width is fixed.
    Builtin(Builtin),
    /// A field already read, whose width is the span it occupied.
    Path(Vec<String>),
}

/// Prefix operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `-`, two's-complement negation.
    Negate,
    /// `~`, bitwise complement.
    Complement,
    /// `!`, logical negation.
    Not,
}

/// Infix operators, in the C precedence the language inherits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `%`
    Remainder,
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `&`
    BitAnd,
    /// `^`
    BitXor,
    /// `|`
    BitOr,
    /// `&&`
    And,
    /// `||`
    Or,
}

impl BinaryOperator {
    /// Binding power, higher binding tighter.
    ///
    /// These are C's levels, which is what the language this grammar follows
    /// uses. Writing them as one table rather than a ladder of parser
    /// functions keeps the precedence readable and in one place.
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Multiply | Self::Divide | Self::Remainder => 10,
            Self::Add | Self::Subtract => 9,
            Self::ShiftLeft | Self::ShiftRight => 8,
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => 7,
            Self::Equal | Self::NotEqual => 6,
            Self::BitAnd => 5,
            Self::BitXor => 4,
            Self::BitOr => 3,
            Self::And => 2,
            Self::Or => 1,
        }
    }
}

/// How many elements an array holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArrayLength {
    /// `[expr]` -- a count computed before the first element is read.
    Counted(Expression),
    /// `[while(expr)]` -- read elements while the test holds.
    While(Expression),
}

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
                || matches!(declaration, Declaration::Union(value) if value.name == name)
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
    /// `union Name { ... }`
    Union(UnionDeclaration),
    /// `enum Name : Type { ... }`
    Enum(EnumDeclaration),
    /// `bitfield Name { ... }`
    Bitfield(BitfieldDeclaration),
    /// `using Alias = Type;`
    Alias(AliasDeclaration),
    /// `Type name @ address;`
    Placement(Placement),
}

/// A composite type whose members are laid out back to back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructDeclaration {
    /// Type name.
    pub name: String,
    /// Members in layout order.
    pub members: Vec<Member>,
}

/// A composite type whose members all begin at the same offset.
///
/// The size is the widest member rather than the sum, and evaluation reads
/// every member from the same address. That is the whole difference from a
/// struct, so the two share their member type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionDeclaration {
    /// Type name.
    pub name: String,
    /// Members, all starting at offset zero within the union.
    pub members: Vec<Member>,
}

/// One entry in a struct or union body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    /// A named, typed field.
    Field(Field),
    /// `if (condition) { ... } else { ... }`, choosing members by a test over
    /// the fields already read.
    Conditional {
        /// The test, evaluated against the members read so far.
        condition: Expression,
        /// Members contributed when the test holds.
        when_true: Vec<Member>,
        /// Members contributed otherwise; empty when there is no `else`.
        when_false: Vec<Member>,
    },
    /// `padding[expr];`, advancing the cursor without producing a field.
    Padding(Expression),
}

/// One named field of a struct or union.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Member name.
    pub name: String,
    /// Member type.
    pub type_reference: TypeReference,
    /// `Some(..)` when the member is an array.
    pub array_length: Option<ArrayLength>,
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
    /// `Some(..)` when the variable is an array.
    pub array_length: Option<ArrayLength>,
    /// Absolute byte offset the variable starts at.
    ///
    /// An expression rather than a literal so a placement can be written
    /// relative to an earlier one -- `Body body @ sizeof(header);` -- which is
    /// how a format with a variable-length header is described.
    pub address: Expression,
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

    /// The spelling this type has in pattern source.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unsigned(1) => "u8",
            Self::Unsigned(2) => "u16",
            Self::Unsigned(3) => "u24",
            Self::Unsigned(4) => "u32",
            Self::Unsigned(6) => "u48",
            Self::Unsigned(8) => "u64",
            Self::Unsigned(12) => "u96",
            Self::Unsigned(_) => "u128",
            Self::Signed(1) => "s8",
            Self::Signed(2) => "s16",
            Self::Signed(3) => "s24",
            Self::Signed(4) => "s32",
            Self::Signed(6) => "s48",
            Self::Signed(8) => "s64",
            Self::Signed(12) => "s96",
            Self::Signed(_) => "s128",
            Self::Float => "float",
            Self::Double => "double",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::Char16 => "char16",
        }
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
