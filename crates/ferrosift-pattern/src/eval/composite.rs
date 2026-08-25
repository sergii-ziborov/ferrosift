use alloc::string::ToString;
use alloc::vec::Vec;

use super::evaluator::Evaluator;
use super::expression::{self, Scope};
use super::value::{Node, NodeValue};
use crate::ast::{
    BitfieldDeclaration, Endian, EnumDeclaration, Member, StructDeclaration, TypeReference,
    UnionDeclaration,
};
use crate::error::PatternError;

/// Whether members follow each other or share one address.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// Each member begins where the previous one ended.
    Sequential,
    /// Every member begins at the composite's own offset.
    Overlaid,
}

/// Lays struct members out back to back from `offset`.
pub(super) fn structure(
    evaluator: &mut Evaluator<'_>,
    name: &str,
    declaration: &StructDeclaration,
    endian: Option<Endian>,
    offset: u64,
    depth: u32,
) -> Result<Node, PatternError> {
    let (children, end) = members(
        evaluator,
        &declaration.members,
        endian,
        offset,
        depth,
        Layout::Sequential,
    )?;
    Ok(Node {
        name: name.to_string(),
        type_name: declaration.name.clone(),
        offset,
        size: end.saturating_sub(offset),
        value: NodeValue::Group(children),
    })
}

/// Reads every union member from the same address.
///
/// The size is the widest member rather than the sum. Members still see the
/// ones declared before them, so a union can be discriminated by a field read
/// earlier in the enclosing struct.
pub(super) fn union(
    evaluator: &mut Evaluator<'_>,
    name: &str,
    declaration: &UnionDeclaration,
    endian: Option<Endian>,
    offset: u64,
    depth: u32,
) -> Result<Node, PatternError> {
    let (children, end) = members(
        evaluator,
        &declaration.members,
        endian,
        offset,
        depth,
        Layout::Overlaid,
    )?;
    Ok(Node {
        name: name.to_string(),
        type_name: declaration.name.clone(),
        offset,
        size: end.saturating_sub(offset),
        value: NodeValue::Group(children),
    })
}

/// Walks a member list, returning the nodes produced and the end offset.
///
/// Conditional members are flattened into the enclosing body rather than
/// producing a node of their own, so `if` changes which fields exist without
/// changing the shape of the value tree.
fn members(
    evaluator: &mut Evaluator<'_>,
    list: &[Member],
    endian: Option<Endian>,
    base: u64,
    depth: u32,
    layout: Layout,
) -> Result<(Vec<Node>, u64), PatternError> {
    let mut children: Vec<Node> = Vec::new();
    let mut end = base;
    walk(
        evaluator,
        list,
        endian,
        base,
        depth,
        layout,
        &mut children,
        &mut end,
    )?;
    Ok((children, end))
}

/// The recursive half of [`members`], so a conditional can contribute into the
/// same node list and cursor as the body that holds it.
#[expect(
    clippy::too_many_arguments,
    reason = "the recursion threads one layout state; splitting it into a struct would name each field twice"
)]
fn walk(
    evaluator: &mut Evaluator<'_>,
    list: &[Member],
    endian: Option<Endian>,
    base: u64,
    depth: u32,
    layout: Layout,
    children: &mut Vec<Node>,
    end: &mut u64,
) -> Result<(), PatternError> {
    for member in list {
        let start = match layout {
            Layout::Sequential => *end,
            Layout::Overlaid => base,
        };
        match member {
            Member::Field(field) => {
                // An explicit prefix on the composite's use propagates to
                // members that do not carry one of their own.
                let field_type = TypeReference {
                    kind: field.type_reference.kind.clone(),
                    endian: field.type_reference.endian.or(endian),
                };
                let scope = Scope {
                    siblings: children,
                    offset: start,
                    pattern: Some(evaluator.pattern),
                };
                let child = evaluator.item(
                    &field.name,
                    &field_type,
                    field.array_length.as_ref(),
                    start,
                    depth + 1,
                    scope,
                )?;
                *end = (*end).max(child.end());
                children.push(child);
            }
            Member::Conditional {
                condition,
                when_true,
                when_false,
            } => {
                let scope = Scope {
                    siblings: children,
                    offset: start,
                    pattern: Some(evaluator.pattern),
                };
                let taken = if expression::evaluate(condition, scope)? == 0 {
                    when_false
                } else {
                    when_true
                };
                walk(
                    evaluator, taken, endian, base, depth, layout, children, end,
                )?;
            }
            Member::Padding(count) => {
                let scope = Scope {
                    siblings: children,
                    offset: start,
                    pattern: Some(evaluator.pattern),
                };
                let count = expression::evaluate(count, scope)?;
                let count = u64::try_from(count).map_err(|_| {
                    super::evaluator::fail(
                        super::evaluator::INVALID_LENGTH,
                        "padding length is negative or too large",
                    )
                })?;
                *end = (*end).max(start.saturating_add(count));
            }
        }
    }
    Ok(())
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

