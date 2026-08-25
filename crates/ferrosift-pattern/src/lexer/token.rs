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
    /// `union`, whose members all start at the same offset.
    Union,
    /// `using`
    Using,
    /// `be`, forcing big-endian reads.
    BigEndian,
    /// `le`, forcing little-endian reads.
    LittleEndian,
    /// `if`
    If,
    /// `else`
    Else,
    /// `while`, sizing an array by a test instead of a count.
    While,
    /// `padding`, reserving bytes without naming them.
    Padding,
    /// `sizeof`, the byte width of a type or a read field.
    Sizeof,
    /// `true`
    True,
    /// `false`
    False,
}

impl Keyword {
    /// Resolves a word to a keyword, or `None` when it is an identifier.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "bitfield" => Self::Bitfield,
            "union" => Self::Union,
            "using" => Self::Using,
            "be" => Self::BigEndian,
            "le" => Self::LittleEndian,
            "if" => Self::If,
            "else" => Self::Else,
            "while" => Self::While,
            "padding" => Self::Padding,
            "sizeof" => Self::Sizeof,
            "true" => Self::True,
            "false" => Self::False,
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
    /// `(`
    ParenOpen,
    /// `)`
    ParenClose,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `::`, qualifying a constant with the enum that declares it.
    PathSep,
    /// `=`
    Assign,
    /// `@`, placing a variable at an absolute offset.
    At,
    /// `.`, selecting a member of a composite value.
    Dot,
    /// `?`, opening a conditional expression.
    Question,
    /// `$`, the offset the current field starts at.
    Dollar,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `&`
    Ampersand,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// `!`
    Bang,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    Less,
    /// `>`
    Greater,
    /// `<=`
    LessEqual,
    /// `>=`
    GreaterEqual,
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
            '(' => Self::ParenOpen,
            ')' => Self::ParenClose,
            ';' => Self::Semicolon,
            ',' => Self::Comma,
            ':' => Self::Colon,
            '=' => Self::Assign,
            '@' => Self::At,
            '.' => Self::Dot,
            '?' => Self::Question,
            '$' => Self::Dollar,
            '+' => Self::Plus,
            '-' => Self::Minus,
            '*' => Self::Star,
            '/' => Self::Slash,
            '%' => Self::Percent,
            '&' => Self::Ampersand,
            '|' => Self::Pipe,
            '^' => Self::Caret,
            '~' => Self::Tilde,
            '!' => Self::Bang,
            '<' => Self::Less,
            '>' => Self::Greater,
            _ => return None,
        })
    }

    /// Resolves a two-character operator, or `None` when the pair is not one.
    ///
    /// Tried before [`Symbol::parse`], because every pair here begins with a
    /// character that is also a symbol on its own: `<` and `<=` differ only in
    /// what follows, and taking the shorter one first would leave a stray `=`.
    #[must_use]
    pub const fn parse_pair(first: char, second: char) -> Option<Self> {
        Some(match (first, second) {
            ('<', '<') => Self::ShiftLeft,
            ('>', '>') => Self::ShiftRight,
            ('&', '&') => Self::AndAnd,
            ('|', '|') => Self::OrOr,
            ('=', '=') => Self::Equal,
            ('!', '=') => Self::NotEqual,
            ('<', '=') => Self::LessEqual,
            ('>', '=') => Self::GreaterEqual,
            (':', ':') => Self::PathSep,
            _ => return None,
        })
    }
}
