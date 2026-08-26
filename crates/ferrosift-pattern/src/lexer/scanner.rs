use alloc::string::String;
use alloc::vec::Vec;

use super::token::{Keyword, Symbol, Token, TokenKind};
use crate::error::{PatternError, Position};

const UNTERMINATED_COMMENT: &str = "pattern.lex.unterminated_comment";
const UNTERMINATED_TEXT: &str = "pattern.lex.unterminated_text";
const INVALID_ESCAPE: &str = "pattern.lex.invalid_escape";
const INVALID_NUMBER: &str = "pattern.lex.invalid_number";
const NUMBER_OVERFLOW: &str = "pattern.lex.number_overflow";
const UNEXPECTED_CHARACTER: &str = "pattern.lex.unexpected_character";

/// Splits pattern source into tokens, ending with [`TokenKind::End`].
///
/// # Errors
///
/// Returns a [`PatternError`] with a stable `pattern.lex.*` code when the
/// source contains an unterminated comment or literal, a malformed escape or
/// number, or a character outside the supported subset.
pub fn scan(source: &str) -> Result<Vec<Token>, PatternError> {
    Scanner::new(source).run()
}

struct Scanner {
    source: Vec<char>,
    index: usize,
    line: u32,
    column: u32,
}

impl Scanner {
    fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn run(mut self) -> Result<Vec<Token>, PatternError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_ignored()?;
            let position = self.position();
            let Some(value) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::End,
                    position,
                });
                return Ok(tokens);
            };
            let kind = if value.is_ascii_digit() {
                self.scan_number()?
            } else if is_identifier_start(value) {
                self.scan_word()
            } else if value == '\'' {
                self.scan_char()?
            } else if value == '"' {
                self.scan_text()?
            } else if let Some(symbol) = self
                .peek_at(1)
                .and_then(|next| Symbol::parse_pair(value, next))
            {
                self.advance();
                self.advance();
                TokenKind::Symbol(symbol)
            } else if let Some(symbol) = Symbol::parse(value) {
                self.advance();
                TokenKind::Symbol(symbol)
            } else {
                return Err(Self::fail(
                    UNEXPECTED_CHARACTER,
                    position,
                    "unsupported character",
                ));
            };
            tokens.push(Token { kind, position });
        }
    }

    fn skip_ignored(&mut self) -> Result<(), PatternError> {
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some(value), _) if value.is_whitespace() => {
                    self.advance();
                }
                (Some('/'), Some('/')) => {
                    while self.peek().is_some_and(|value| value != '\n') {
                        self.advance();
                    }
                }
                (Some('/'), Some('*')) => self.skip_block_comment()?,
                _ => return Ok(()),
            }
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), PatternError> {
        let position = self.position();
        self.advance();
        self.advance();
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some('*'), Some('/')) => {
                    self.advance();
                    self.advance();
                    return Ok(());
                }
                (Some(_), _) => self.advance(),
                (None, _) => {
                    return Err(Self::fail(
                        UNTERMINATED_COMMENT,
                        position,
                        "block comment is never closed",
                    ));
                }
            }
        }
    }

    fn scan_word(&mut self) -> TokenKind {
        let mut word = String::new();
        while let Some(value) = self.peek() {
            if !is_identifier_part(value) {
                break;
            }
            word.push(value);
            self.advance();
        }
        Keyword::parse(&word).map_or(TokenKind::Identifier(word), TokenKind::Keyword)
    }

    fn scan_number(&mut self) -> Result<TokenKind, PatternError> {
        let position = self.position();
        let radix = self.scan_radix();
        let mut digits = String::new();
        while let Some(value) = self.peek() {
            if value == '_' {
                self.advance();
                continue;
            }
            if !value.is_ascii_alphanumeric() {
                break;
            }
            digits.push(value);
            self.advance();
        }
        if digits.is_empty() {
            return Err(Self::fail(INVALID_NUMBER, position, "number has no digits"));
        }
        let mut value: u128 = 0;
        for digit in digits.chars() {
            let parsed = digit
                .to_digit(radix)
                .ok_or_else(|| Self::fail(INVALID_NUMBER, position, "digit outside the radix"))?;
            value = value
                .checked_mul(u128::from(radix))
                .and_then(|shifted| shifted.checked_add(u128::from(parsed)))
                .ok_or_else(|| Self::fail(NUMBER_OVERFLOW, position, "number exceeds 128 bits"))?;
        }
        Ok(TokenKind::Integer(value))
    }

    /// Consumes a `0x` / `0b` / `0o` prefix and reports the radix it selects.
    fn scan_radix(&mut self) -> u32 {
        if self.peek() != Some('0') {
            return 10;
        }
        let radix = match self.peek_at(1) {
            Some('x' | 'X') => 16,
            Some('b' | 'B') => 2,
            Some('o' | 'O') => 8,
            _ => return 10,
        };
        self.advance();
        self.advance();
        radix
    }

    fn scan_char(&mut self) -> Result<TokenKind, PatternError> {
        let position = self.position();
        self.advance();
        let value = match self.peek() {
            Some('\\') => self.scan_escape(position)?,
            Some('\'') | None => {
                return Err(Self::fail(
                    UNTERMINATED_TEXT,
                    position,
                    "empty character literal",
                ));
            }
            Some(value) => {
                self.advance();
                value
            }
        };
        if self.peek() != Some('\'') {
            return Err(Self::fail(
                UNTERMINATED_TEXT,
                position,
                "character literal is never closed",
            ));
        }
        self.advance();
        Ok(TokenKind::Char(value))
    }

    fn scan_text(&mut self) -> Result<TokenKind, PatternError> {
        let position = self.position();
        self.advance();
        let mut text = String::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return Ok(TokenKind::Text(text));
                }
                Some('\\') => text.push(self.scan_escape(position)?),
                Some(value) => {
                    text.push(value);
                    self.advance();
                }
                None => {
                    return Err(Self::fail(
                        UNTERMINATED_TEXT,
                        position,
                        "string literal is never closed",
                    ));
                }
            }
        }
    }

    fn scan_escape(&mut self, position: Position) -> Result<char, PatternError> {
        self.advance();
        let Some(value) = self.peek() else {
            return Err(Self::fail(
                INVALID_ESCAPE,
                position,
                "escape has no character",
            ));
        };
        self.advance();
        Ok(match value {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            '\\' => '\\',
            '\'' => '\'',
            '"' => '"',
            _ => return Err(Self::fail(INVALID_ESCAPE, position, "unsupported escape")),
        })
    }

    fn peek(&self) -> Option<char> {
        self.peek_at(0)
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.source.get(self.index + offset).copied()
    }

    fn advance(&mut self) {
        if let Some(value) = self.peek() {
            self.index += 1;
            if value == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    const fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }

    fn fail(code: &'static str, position: Position, detail: &str) -> PatternError {
        PatternError::new(code, position, detail)
    }
}

fn is_identifier_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_identifier_part(value: char) -> bool {
    value == '_' || value.is_ascii_alphanumeric()
}
