use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::cursor::{
    Cursor, DUPLICATE_DECLARATION, EXPECTED_TYPE, INVALID_ARRAY_LENGTH, INVALID_BIT_WIDTH,
    UNEXPECTED_TOKEN,
};
use crate::ast::{
    AliasDeclaration, BitfieldDeclaration, BitfieldMember, Builtin, Declaration, Endian,
    EnumDeclaration, EnumEntry, Field, Pattern, Placement, StructDeclaration, TypeKind,
    TypeReference,
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
    cursor.expect(Symbol::BraceOpen)?;
    let mut fields = Vec::new();
    while !cursor.eat(Symbol::BraceClose) {
        if cursor.at_end() {
            return Err(cursor.fail(UNEXPECTED_TOKEN, "struct body is never closed"));
        }
        fields.push(field(cursor)?);
    }
    cursor.eat(Symbol::Semicolon);
    Ok(StructDeclaration { name, fields })
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
            cursor.expect_integer()?
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
        let bits = cursor.expect_integer()?;
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
    let address = cursor.expect_integer()?;
    cursor.expect(Symbol::Semicolon)?;
    Ok(Placement {
        name,
        type_reference,
        array_length,
        address,
    })
}

fn array_length(cursor: &mut Cursor) -> Result<Option<u128>, PatternError> {
    if !cursor.eat(Symbol::BracketOpen) {
        return Ok(None);
    }
    let length = cursor.expect_integer()?;
    if length == 0 {
        return Err(cursor.fail(INVALID_ARRAY_LENGTH, "array length must be positive"));
    }
    cursor.expect(Symbol::BracketClose)?;
    Ok(Some(length))
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
