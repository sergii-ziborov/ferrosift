use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::cursor::{
    Cursor, DUPLICATE_DECLARATION, EXPECTED_INTEGER, EXPECTED_TYPE, INVALID_ARRAY_LENGTH,
    INVALID_BIT_WIDTH, UNEXPECTED_TOKEN,
};
use super::expression::expression;
use crate::ast::{
    AliasDeclaration, ArrayLength, BitfieldDeclaration, BitfieldMember, Builtin, Declaration,
    Endian, EnumDeclaration, EnumEntry, Expression, Field, Member, Pattern, Placement,
    StructDeclaration, TypeKind, TypeReference, UnionDeclaration,
};
use crate::error::PatternError;
use crate::lexer::{Keyword, Symbol, TokenKind};

pub(super) fn pattern(cursor: &mut Cursor) -> Result<Pattern, PatternError> {
    let mut declarations = Vec::new();
    let mut names = BTreeSet::new();
    while !cursor.at_end() {
        let declaration = self::declaration(cursor)?;
        let name = declared_name(&declaration);
        if !names.insert(name.clone()) {
            return Err(cursor.fail(
                DUPLICATE_DECLARATION,
                format!("`{name}` is declared more than once"),
            ));
        }
        declarations.push(declaration);
    }
    Ok(Pattern { declarations })
}

fn declared_name(declaration: &Declaration) -> String {
    match declaration {
        Declaration::Struct(value) => value.name.clone(),
        Declaration::Union(value) => value.name.clone(),
        Declaration::Enum(value) => value.name.clone(),
        Declaration::Bitfield(value) => value.name.clone(),
        Declaration::Alias(value) => value.name.clone(),
        Declaration::Placement(value) => value.name.clone(),
    }
}

fn declaration(cursor: &mut Cursor) -> Result<Declaration, PatternError> {
    if cursor.eat_keyword(Keyword::Struct) {
        return structure(cursor).map(Declaration::Struct);
    }
    if cursor.eat_keyword(Keyword::Union) {
        return union(cursor).map(Declaration::Union);
    }
    if cursor.eat_keyword(Keyword::Enum) {
        return enumeration(cursor).map(Declaration::Enum);
    }
    if cursor.eat_keyword(Keyword::Bitfield) {
        return bitfield(cursor).map(Declaration::Bitfield);
    }
    if cursor.eat_keyword(Keyword::Using) {
        return alias(cursor).map(Declaration::Alias);
    }
    placement(cursor).map(Declaration::Placement)
}

fn structure(cursor: &mut Cursor) -> Result<StructDeclaration, PatternError> {
    let name = cursor.expect_identifier()?;
    let members = body(cursor, "struct")?;
    cursor.eat(Symbol::Semicolon);
    Ok(StructDeclaration { name, members })
}

fn union(cursor: &mut Cursor) -> Result<UnionDeclaration, PatternError> {
    let name = cursor.expect_identifier()?;
    let members = body(cursor, "union")?;
    cursor.eat(Symbol::Semicolon);
    Ok(UnionDeclaration { name, members })
}

/// Reads a brace-delimited member list, shared by structs, unions, and the
/// arms of a conditional.
fn body(cursor: &mut Cursor, what: &'static str) -> Result<Vec<Member>, PatternError> {
    cursor.expect(Symbol::BraceOpen)?;
    let mut members = Vec::new();
    while !cursor.eat(Symbol::BraceClose) {
        if cursor.at_end() {
            return Err(cursor.fail(
                UNEXPECTED_TOKEN,
                format!("{what} body is never closed"),
            ));
        }
        members.push(member(cursor)?);
    }
    Ok(members)
}

fn member(cursor: &mut Cursor) -> Result<Member, PatternError> {
    if cursor.eat_keyword(Keyword::If) {
        return conditional(cursor);
    }
    if cursor.eat_keyword(Keyword::Padding) {
        cursor.expect(Symbol::BracketOpen)?;
        let count = expression(cursor)?;
        cursor.expect(Symbol::BracketClose)?;
        cursor.expect(Symbol::Semicolon)?;
        return Ok(Member::Padding(count));
    }
    field(cursor).map(Member::Field)
}

/// `if (condition) { ... }` with an optional `else`, which may be another
/// `if` so that a chain reads the way it is written.
fn conditional(cursor: &mut Cursor) -> Result<Member, PatternError> {
    cursor.expect(Symbol::ParenOpen)?;
    let condition = expression(cursor)?;
    cursor.expect(Symbol::ParenClose)?;
    let when_true = body(cursor, "if")?;
    let when_false = if cursor.eat_keyword(Keyword::Else) {
        if cursor.eat_keyword(Keyword::If) {
            vec![conditional(cursor)?]
        } else {
            body(cursor, "else")?
        }
    } else {
        Vec::new()
    };
    Ok(Member::Conditional {
        condition,
        when_true,
        when_false,
    })
}

fn field(cursor: &mut Cursor) -> Result<Field, PatternError> {
    let type_reference = type_reference(cursor)?;
    let name = cursor.expect_identifier()?;
    let array_length = array_length(cursor)?;
    cursor.expect(Symbol::Semicolon)?;
    Ok(Field {
        name,
        type_reference,
        array_length,
    })
}

fn enumeration(cursor: &mut Cursor) -> Result<EnumDeclaration, PatternError> {
    let name = cursor.expect_identifier()?;
    cursor.expect(Symbol::Colon)?;
    let backing = type_reference(cursor)?;
    cursor.expect(Symbol::BraceOpen)?;
    let mut entries = Vec::new();
    let mut next = 0_u128;
    while !cursor.eat(Symbol::BraceClose) {
        if cursor.at_end() {
            return Err(cursor.fail(UNEXPECTED_TOKEN, "enum body is never closed"));
        }
        let entry_name = cursor.expect_identifier()?;
        let value = if cursor.eat(Symbol::Assign) {
            constant(cursor, EXPECTED_INTEGER, "enum value")?
        } else {
            next
        };
        next = value.saturating_add(1);
        entries.push(EnumEntry {
            name: entry_name,
            value,
        });
        if !cursor.eat(Symbol::Comma) {
            cursor.expect(Symbol::BraceClose)?;
            break;
        }
    }
    cursor.eat(Symbol::Semicolon);
    Ok(EnumDeclaration {
        name,
        backing,
        entries,
    })
}

fn bitfield(cursor: &mut Cursor) -> Result<BitfieldDeclaration, PatternError> {
    let name = cursor.expect_identifier()?;
    cursor.expect(Symbol::BraceOpen)?;
    let mut members = Vec::new();
    while !cursor.eat(Symbol::BraceClose) {
        if cursor.at_end() {
            return Err(cursor.fail(UNEXPECTED_TOKEN, "bitfield body is never closed"));
        }
        let member_name = cursor.expect_identifier()?;
        cursor.expect(Symbol::Colon)?;
        let bits = constant(cursor, INVALID_BIT_WIDTH, "bit width")?;
        let bits = u32::try_from(bits)
            .ok()
            .filter(|width| (1..=64).contains(width))
            .ok_or_else(|| cursor.fail(INVALID_BIT_WIDTH, "bit width must be between 1 and 64"))?;
        cursor.expect(Symbol::Semicolon)?;
        members.push(BitfieldMember {
            name: member_name,
            bits,
        });
    }
    cursor.eat(Symbol::Semicolon);
    Ok(BitfieldDeclaration { name, members })
}

fn alias(cursor: &mut Cursor) -> Result<AliasDeclaration, PatternError> {
    let name = cursor.expect_identifier()?;
    cursor.expect(Symbol::Assign)?;
    let target = type_reference(cursor)?;
    cursor.expect(Symbol::Semicolon)?;
    Ok(AliasDeclaration { name, target })
}

fn placement(cursor: &mut Cursor) -> Result<Placement, PatternError> {
    let type_reference = type_reference(cursor)?;
    let name = cursor.expect_identifier()?;
    let array_length = array_length(cursor)?;
    cursor.expect(Symbol::At)?;
    let address = expression(cursor)?;
    cursor.expect(Symbol::Semicolon)?;
    Ok(Placement {
        name,
        type_reference,
        array_length,
        address,
    })
}

/// `[expr]` or `[while(expr)]`, or nothing at all.
///
/// A literal count is checked here, where the position is still known. A
/// computed one cannot be: its value depends on bytes that have not been read,
/// so a negative or oversized result is an evaluation failure instead.
fn array_length(cursor: &mut Cursor) -> Result<Option<ArrayLength>, PatternError> {
    if !cursor.eat(Symbol::BracketOpen) {
        return Ok(None);
    }
    if cursor.eat_keyword(Keyword::While) {
        cursor.expect(Symbol::ParenOpen)?;
        let condition = expression(cursor)?;
        cursor.expect(Symbol::ParenClose)?;
        cursor.expect(Symbol::BracketClose)?;
        return Ok(Some(ArrayLength::While(condition)));
    }
    let count = expression(cursor)?;
    if count == Expression::Integer(0) {
        return Err(cursor.fail(INVALID_ARRAY_LENGTH, "array length must be positive"));
    }
    cursor.expect(Symbol::BracketClose)?;
    Ok(Some(ArrayLength::Counted(count)))
}

/// Reads an expression that must resolve without reading any data.
///
/// Enum values and bit widths are fixed by the source, so allowing an
/// expression there costs nothing at evaluation time -- `Flag = 1 << 3` folds
/// once, here, and the result is what every read compares against.
fn constant(
    cursor: &mut Cursor,
    code: &'static str,
    what: &'static str,
) -> Result<u128, PatternError> {
    let value = expression(cursor)?;
    let folded = crate::eval::fold(&value)
        .map_err(|_| cursor.fail(code, format!("{what} must be a constant expression")))?;
    u128::try_from(folded)
        .map_err(|_| cursor.fail(code, format!("{what} must not be negative")))
}

fn type_reference(cursor: &mut Cursor) -> Result<TypeReference, PatternError> {
    let endian = if cursor.eat_keyword(Keyword::BigEndian) {
        Some(Endian::Big)
    } else if cursor.eat_keyword(Keyword::LittleEndian) {
        Some(Endian::Little)
    } else {
        None
    };
    let TokenKind::Identifier(word) = cursor.peek() else {
        return Err(cursor.fail(EXPECTED_TYPE, "expected a type name"));
    };
    let word = word.clone();
    cursor.advance();
    let kind = Builtin::parse(&word).map_or(TypeKind::Named(word), TypeKind::Builtin);
    Ok(TypeReference { kind, endian })
}
