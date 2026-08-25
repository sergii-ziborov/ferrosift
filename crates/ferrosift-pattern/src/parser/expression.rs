//! Parsing expressions by precedence climbing.
//!
//! One loop drives every infix level, consulting
//! [`BinaryOperator::precedence`] rather than descending through a function
//! per level. The ladder version needs a new function each time an operator is
//! added and puts the precedence in the call graph, where it cannot be read;
//! this keeps it in one table next to the operators it orders.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::cursor::{Cursor, EXPECTED_TYPE, UNEXPECTED_TOKEN};
use crate::ast::{
    BinaryOperator, Builtin, Expression, SizeOfTarget, UnaryOperator,
};
use crate::error::PatternError;
use crate::lexer::{Keyword, Symbol, TokenKind};

/// Parses a full expression, including the conditional operator.
pub(super) fn expression(cursor: &mut Cursor) -> Result<Expression, PatternError> {
    conditional(cursor)
}

/// `condition ? a : b`, right-associative so `a ? b : c ? d : e` nests right.
fn conditional(cursor: &mut Cursor) -> Result<Expression, PatternError> {
    let condition = binary(cursor, 0)?;
    if !cursor.eat(Symbol::Question) {
        return Ok(condition);
    }
    let when_true = conditional(cursor)?;
    cursor.expect(Symbol::Colon)?;
    let when_false = conditional(cursor)?;
    Ok(Expression::Conditional {
        condition: Box::new(condition),
        when_true: Box::new(when_true),
        when_false: Box::new(when_false),
    })
}

/// Consumes infix operators binding at least as tightly as `floor`.
fn binary(cursor: &mut Cursor, floor: u8) -> Result<Expression, PatternError> {
    let mut left = unary(cursor)?;
    while let Some(operator) = peek_operator(cursor) {
        let precedence = operator.precedence();
        if precedence < floor {
            break;
        }
        cursor.advance();
        // Every operator here is left-associative, so the right operand stops
        // at anything binding equally -- one level above this operator's own.
        let right = binary(cursor, precedence + 1)?;
        left = Expression::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    Ok(left)
}

/// Reads the infix operator at the cursor without consuming it.
fn peek_operator(cursor: &Cursor) -> Option<BinaryOperator> {
    let TokenKind::Symbol(symbol) = cursor.peek() else {
        return None;
    };
    Some(match symbol {
        Symbol::Star => BinaryOperator::Multiply,
        Symbol::Slash => BinaryOperator::Divide,
        Symbol::Percent => BinaryOperator::Remainder,
        Symbol::Plus => BinaryOperator::Add,
        Symbol::Minus => BinaryOperator::Subtract,
        Symbol::ShiftLeft => BinaryOperator::ShiftLeft,
        Symbol::ShiftRight => BinaryOperator::ShiftRight,
        Symbol::Less => BinaryOperator::Less,
        Symbol::LessEqual => BinaryOperator::LessEqual,
        Symbol::Greater => BinaryOperator::Greater,
        Symbol::GreaterEqual => BinaryOperator::GreaterEqual,
        Symbol::Equal => BinaryOperator::Equal,
        Symbol::NotEqual => BinaryOperator::NotEqual,
        Symbol::Ampersand => BinaryOperator::BitAnd,
        Symbol::Caret => BinaryOperator::BitXor,
        Symbol::Pipe => BinaryOperator::BitOr,
        Symbol::AndAnd => BinaryOperator::And,
        Symbol::OrOr => BinaryOperator::Or,
        _ => return None,
    })
}

/// Prefix operators, which bind tighter than every infix one.
fn unary(cursor: &mut Cursor) -> Result<Expression, PatternError> {
    let operator = match cursor.peek() {
        TokenKind::Symbol(Symbol::Minus) => UnaryOperator::Negate,
        TokenKind::Symbol(Symbol::Tilde) => UnaryOperator::Complement,
        TokenKind::Symbol(Symbol::Bang) => UnaryOperator::Not,
        _ => return primary(cursor),
    };
    cursor.advance();
    Ok(Expression::Unary {
        operator,
        operand: Box::new(unary(cursor)?),
    })
}

/// Literals, paths, `$`, `sizeof`, and parenthesised expressions.
fn primary(cursor: &mut Cursor) -> Result<Expression, PatternError> {
    match cursor.peek().clone() {
        TokenKind::Integer(value) => {
            cursor.advance();
            Ok(Expression::Integer(value))
        }
        TokenKind::Char(value) => {
            cursor.advance();
            Ok(Expression::Char(value))
        }
        TokenKind::Keyword(Keyword::True) => {
            cursor.advance();
            Ok(Expression::Bool(true))
        }
        TokenKind::Keyword(Keyword::False) => {
            cursor.advance();
            Ok(Expression::Bool(false))
        }
        TokenKind::Symbol(Symbol::Dollar) => {
            cursor.advance();
            Ok(Expression::Offset)
        }
        TokenKind::Symbol(Symbol::ParenOpen) => {
            cursor.advance();
            let inner = expression(cursor)?;
            cursor.expect(Symbol::ParenClose)?;
            Ok(inner)
        }
        TokenKind::Keyword(Keyword::Sizeof) => {
            cursor.advance();
            size_of(cursor)
        }
        TokenKind::Identifier(_) => Ok(Expression::Path(path(cursor)?)),
        _ => Err(cursor.fail(UNEXPECTED_TOKEN, "expected a value")),
    }
}

/// `sizeof(u32)` or `sizeof(header.length)`.
///
/// A built-in names a width the grammar already knows. A path names a field
/// that has been read, whose width is the span it actually occupied -- which
/// is the only way to ask the size of something whose length varied.
fn size_of(cursor: &mut Cursor) -> Result<Expression, PatternError> {
    cursor.expect(Symbol::ParenOpen)?;
    let TokenKind::Identifier(word) = cursor.peek().clone() else {
        return Err(cursor.fail(EXPECTED_TYPE, "expected a type or field name"));
    };
    let target = if let Some(builtin) = Builtin::parse(&word) {
        cursor.advance();
        SizeOfTarget::Builtin(builtin)
    } else {
        SizeOfTarget::Path(path(cursor)?)
    };
    cursor.expect(Symbol::ParenClose)?;
    Ok(Expression::SizeOf(target))
}

/// A dotted path: one identifier, then any number of `.name` selectors.
fn path(cursor: &mut Cursor) -> Result<Vec<String>, PatternError> {
    let mut segments = vec![cursor.expect_identifier()?];
    while cursor.eat(Symbol::Dot) {
        segments.push(cursor.expect_identifier()?);
    }
    Ok(segments)
}
