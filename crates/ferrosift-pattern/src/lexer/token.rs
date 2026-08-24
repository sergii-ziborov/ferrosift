use alloc::string::String;

use crate::error::Position;

/// A lexical token together with the source position it started at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// What the token is.
    pub kind: TokenKind,
    /// Where the token starts in the source.
    pub position: Position,
}

/// Every token the pattern subset recognises.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// A reserved word.
    Keyword(Keyword),
    /// A user-defined name.
    Identifier(String),
    /// An integer literal, already folded to its numeric value.
    Integer(u128),
    /// A character literal, folded to its code point.
    Char(char),
    /// A string literal with escapes resolved.
    Text(String),
    /// A punctuation mark or operator.
    Symbol(Symbol),
    /// End of input.
    End,
}

/// Reserved words in the supported subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyword {
    /// `struct`
    Struct,
    /// `enum`
    Enum,
    /// `bitfield`
    Bitfield,
    /// `using`
    Using,
    /// `be`, forcing big-endian reads.
    BigEndian,
    /// `le`, forcing little-endian reads.
    LittleEndian,
}

impl Keyword {
    /// Resolves a word to a keyword, or `None` when it is an identifier.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "bitfield" => Self::Bitfield,
            "using" => Self::Using,
            "be" => Self::BigEndian,
            "le" => Self::LittleEndian,
            _ => return None,
        })
    }
}

/// Punctuation in the supported subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    /// `{`
    BraceOpen,
    /// `}`
    BraceClose,
    /// `[`
    BracketOpen,
    /// `]`
    BracketClose,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `=`
    Assign,
    /// `@`, placing a variable at an absolute offset.
    At,
}

impl Symbol {
    /// Resolves a single character to a symbol, or `None` when unsupported.
    #[must_use]
    pub const fn parse(value: char) -> Option<Self> {
        Some(match value {
            '{' => Self::BraceOpen,
            '}' => Self::BraceClose,
            '[' => Self::BracketOpen,
            ']' => Self::BracketClose,
            ';' => Self::Semicolon,
            ',' => Self::Comma,
            ':' => Self::Colon,
            '=' => Self::Assign,
            '@' => Self::At,
            _ => return None,
        })
    }
}
