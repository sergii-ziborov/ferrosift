use alloc::string::ToString;
use alloc::vec::Vec;

use super::evaluator::Evaluator;
use super::value::{Node, NodeValue};
use crate::ast::{BitfieldDeclaration, Endian, EnumDeclaration, StructDeclaration, TypeReference};
use crate::error::PatternError;

/// Lays struct fields out back to back from `offset`.
pub(super) fn structure(
    evaluator: &mut Evaluator<'_>,
    name: &str,
    declaration: &StructDeclaration,
    endian: Option<Endian>,
    offset: u64,
    depth: u32,
) -> Result<Node, PatternError> {
    let mut children = Vec::new();
    let mut cursor = offset;
    for field in &declaration.fields {
        // An explicit prefix on the struct's use propagates to fields that do
        // not carry one of their own.
        let field_type = TypeReference {
            kind: field.type_reference.kind.clone(),
            endian: field.type_reference.endian.or(endian),
        };
        let child = evaluator.item(
            &field.name,
            &field_type,
            field.array_length,
            cursor,
            depth + 1,
        )?;
        cursor = child.end();
        children.push(child);
    }
    Ok(Node {
        name: name.to_string(),
        type_name: declaration.name.clone(),
        offset,
        size: cursor.saturating_sub(offset),
        value: NodeValue::Group(children),
    })
}

/// Reads the backing integer and resolves it against the declared constants.
pub(super) fn enumeration(
    evaluator: &mut Evaluator<'_>,
    name: &str,
    declaration: &EnumDeclaration,
    endian: Option<Endian>,
    offset: u64,
) -> Result<Node, PatternError> {
    let backing = TypeReference {
        kind: declaration.backing.kind.clone(),
        endian: declaration.backing.endian.or(endian),
    };
    let raw = evaluator.value(name, &backing, offset, 0)?;
    let value = match raw.value {
        NodeValue::Unsigned(value) => value,
        NodeValue::Signed(value) => value.cast_unsigned(),
        _ => 0,
    };
    let resolved = declaration
        .entries
        .iter()
        .find(|entry| entry.value == value)
        .map(|entry| entry.name.clone());
    Ok(Node {
        name: name.to_string(),
        type_name: declaration.name.clone(),
        offset,
        size: raw.size,
        value: NodeValue::Enumerator {
            name: resolved,
            value,
        },
    })
}

/// Unpacks bit members from the smallest byte span that holds them.
///
/// Members are extracted most-significant-bit first from a big-endian view of
/// that span, which is the layout this crate defines for the subset.
pub(super) fn bitfield(
    evaluator: &mut Evaluator<'_>,
    name: &str,
    declaration: &BitfieldDeclaration,
    endian: Option<Endian>,
    offset: u64,
) -> Result<Node, PatternError> {
    let total_bits: u32 = declaration
        .members
        .iter()
        .map(|member| member.bits)
        .fold(0, u32::saturating_add);
    let size = total_bits.div_ceil(8);
    let storage = evaluator
        .reader
        .unsigned(offset, size, endian.unwrap_or(Endian::Big))?;

    let mut children = Vec::new();
    let mut consumed = 0_u32;
    for member in &declaration.members {
        evaluator.charge()?;
        let shift = (size * 8).saturating_sub(consumed + member.bits);
        let mask = if member.bits >= 128 {
            u128::MAX
        } else {
            (1_u128 << member.bits) - 1
        };
        children.push(Node {
            name: member.name.clone(),
            type_name: bit_width_name(member.bits),
            offset,
            size: u64::from(size),
            value: NodeValue::Unsigned((storage >> shift) & mask),
        });
        consumed += member.bits;
    }
    Ok(Node {
        name: name.to_string(),
        type_name: declaration.name.clone(),
        offset,
        size: u64::from(size),
        value: NodeValue::Group(children),
    })
}

fn bit_width_name(bits: u32) -> alloc::string::String {
    alloc::format!("{bits} bits")
}
