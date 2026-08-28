use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::error::{PatternError, Position};
use crate::lexer::{Keyword, Symbol, Token, TokenKind};

pub(super) const UNEXPECTED_TOKEN: &str = "pattern.parse.unexpected_token";
pub(super) const EXPECTED_IDENTIFIER: &str = "pattern.parse.expected_identifier";
pub(super) const EXPECTED_SYMBOL: &str = "pattern.parse.expected_symbol";
pub(super) const EXPECTED_INTEGER: &str = "pattern.parse.expected_integer";
pub(super) const EXPECTED_TYPE: &str = "pattern.parse.expected_type";
pub(super) const INVALID_ARRAY_LENGTH: &str = "pattern.parse.invalid_array_length";
pub(super) const INVALID_BIT_WIDTH: &str = "pattern.parse.invalid_bit_width";
pub(super) const DUPLICATE_DECLARATION: &str = "pattern.parse.duplicate_declaration";
/// A #-line this crate cannot honour, which today means #include.
///
/// Named rather than folded into `unexpected_token`, because it is a limit of
/// where this crate runs and not of what the language is: #include names
/// another file and there is no filesystem here to fetch one from.
pub(super) const UNSUPPORTED_DIRECTIVE: &str = "pattern.parse.unsupported_directive";

/// A forward-only cursor over the token stream with expectation helpers.
pub(super) struct Cursor {
    tokens: Vec<Token>,
    index: usize,
}

impl Cursor {
    pub(super) const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    pub(super) fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.index)
            .map_or(&TokenKind::End, |token| &token.kind)
    }

    pub(super) fn position(&self) -> Position {
        self.tokens
            .get(self.index)
            .map_or(Position { line: 1, column: 1 }, |token| token.position)
    }

    pub(super) fn at_end(&self) -> bool {
        matches!(self.peek(), TokenKind::End)
    }

    pub(super) fn advance(&mut self) {
        if self.index < self.tokens.len() {
            self.index += 1;
        }
    }

    /// Consumes the next token when it is `symbol`, reporting whether it was.
    pub(super) fn eat(&mut self, symbol: Symbol) -> bool {
        if self.peek() == &TokenKind::Symbol(symbol) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consumes the next token when it is `keyword`, reporting whether it was.
    pub(super) fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        if self.peek() == &TokenKind::Keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn expect(&mut self, symbol: Symbol) -> Result<(), PatternError> {
        if self.eat(symbol) {
            Ok(())
        } else {
            Err(self.fail(EXPECTED_SYMBOL, format!("expected {symbol:?}")))
        }
    }

    pub(super) fn expect_identifier(&mut self) -> Result<String, PatternError> {
        if let TokenKind::Identifier(name) = self.peek() {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.fail(EXPECTED_IDENTIFIER, "expected a name"))
        }
    }

    pub(super) fn fail(&self, code: &'static str, detail: impl Into<String>) -> PatternError {
        PatternError::new(code, self.position(), detail)
    }
}
