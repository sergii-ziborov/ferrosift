use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::composite;
use super::expression::{self, Scope};
use super::options::EvalOptions;
use super::reader::{Reader, out_of_bounds};
use super::value::{Node, NodeValue};
use crate::ast::{ArrayLength, Builtin, Declaration, Endian, Pattern, TypeKind, TypeReference};
use crate::error::{PatternError, Position};

pub(super) const UNKNOWN_TYPE: &str = "pattern.eval.unknown_type";
pub(super) const DEPTH_EXCEEDED: &str = "pattern.eval.depth_exceeded";
pub(super) const NODE_BUDGET: &str = "pattern.eval.node_budget_exceeded";
pub(super) const INVALID_LENGTH: &str = "pattern.eval.invalid_length";

/// Evaluates every placement in `pattern` against `data`.
///
/// # Errors
///
/// Returns a [`PatternError`] with a stable `pattern.eval.*` code when a read
/// leaves the data, a type name is undeclared, or the pattern exceeds the
/// nesting or node bounds in `options`.
pub fn evaluate(
    pattern: &Pattern,
    data: &[u8],
    options: &EvalOptions,
) -> Result<Vec<Node>, PatternError> {
    let mut evaluator = Evaluator {
        pattern,
        reader: Reader::new(data),
        options: *options,
        nodes: 0,
    };
    let mut roots: Vec<Node> = Vec::new();
    for declaration in &pattern.declarations {
        if let Declaration::Placement(placement) = declaration {
            // Earlier placements are in scope, so an address can be written
            // relative to one already read rather than as a literal.
            let scope = Scope {
                siblings: &roots,
                offset: 0,
            };
            let address = expression::evaluate(&placement.address, scope)?;
            let offset = u64::try_from(address)
                .map_err(|_| out_of_bounds("placement address is past the addressable range"))?;
            let scope = Scope {
                siblings: &roots,
                offset,
            };
            let node = evaluator.item(
                &placement.name,
                &placement.type_reference,
                placement.array_length.as_ref(),
                offset,
                0,
                scope,
            )?;
            roots.push(node);
        }
    }
    Ok(roots)
}

pub(super) struct Evaluator<'a> {
    pub(super) pattern: &'a Pattern,
    pub(super) reader: Reader<'a>,
    pub(super) options: EvalOptions,
    nodes: u64,
}

impl Evaluator<'_> {
    /// Evaluates one named item, which may be an array.
    pub(super) fn item(
        &mut self,
        name: &str,
        type_reference: &TypeReference,
        array_length: Option<&ArrayLength>,
        offset: u64,
        depth: u32,
        scope: Scope<'_>,
    ) -> Result<Node, PatternError> {
        let Some(length) = array_length else {
            return self.value(name, type_reference, offset, depth);
        };
        self.charge()?;
        let mut children = Vec::new();
        let mut cursor = offset;

        match length {
            ArrayLength::Counted(count) => {
                let count = expression::evaluate(count, scope)?;
                let count = u64::try_from(count)
                    .map_err(|_| fail(INVALID_LENGTH, "array length is negative or too large"))?;
                for index in 0..count {
                    let child = self.value(
                        &format!("{name}[{index}]"),
                        type_reference,
                        cursor,
                        depth + 1,
                    )?;
                    cursor = child.end();
                    children.push(child);
                }
            }
            ArrayLength::While(condition) => {
                // The test sees `$` as the offset the next element would start
                // at, which is what makes `[while($ < end)]` mean what it
                // reads as. An element of zero width would spin here; the node
                // budget is what stops it, the same bound that stops a wrong
                // count.
                let mut index: u64 = 0;
                loop {
                    let test = Scope {
                        siblings: scope.siblings,
                        offset: cursor,
                    };
                    if expression::evaluate(condition, test)? == 0 {
                        break;
                    }
                    let child = self.value(
                        &format!("{name}[{index}]"),
                        type_reference,
                        cursor,
                        depth + 1,
                    )?;
                    cursor = child.end();
                    children.push(child);
                    index += 1;
                }
            }
        }

        let count = children.len();
        Ok(Node {
            name: name.to_string(),
            type_name: format!("{}[{count}]", type_name(type_reference)),
            offset,
            size: cursor.saturating_sub(offset),
            value: NodeValue::Group(children),
        })
    }

    /// Evaluates one non-array value of the given type.
    pub(super) fn value(
        &mut self,
        name: &str,
        type_reference: &TypeReference,
        offset: u64,
        depth: u32,
    ) -> Result<Node, PatternError> {
        if depth > self.options.max_depth {
            return Err(fail(DEPTH_EXCEEDED, "type nesting is too deep"));
        }
        self.charge()?;
        let endian = type_reference.endian.unwrap_or(self.options.endian);
        match &type_reference.kind {
            TypeKind::Builtin(builtin) => self.scalar(name, *builtin, endian, offset),
            TypeKind::Named(type_name) => {
                self.named(name, type_name, type_reference.endian, offset, depth)
            }
        }
    }

    fn named(
        &mut self,
        name: &str,
        type_name: &str,
        endian: Option<Endian>,
        offset: u64,
        depth: u32,
    ) -> Result<Node, PatternError> {
        let declaration = self
            .pattern
            .type_named(type_name)
            .ok_or_else(|| fail(UNKNOWN_TYPE, "type is not declared in this pattern"))?;
        match declaration {
            Declaration::Alias(alias) => {
                // An explicit prefix on the alias use wins over the aliased
                // type's own prefix, which in turn wins over the default.
                let target = TypeReference {
                    kind: alias.target.kind.clone(),
                    endian: endian.or(alias.target.endian),
                };
                let mut node = self.value(name, &target, offset, depth + 1)?;
                node.type_name.clone_from(&alias.name);
                Ok(node)
            }
            Declaration::Struct(structure) => {
                composite::structure(self, name, structure, endian, offset, depth)
            }
            Declaration::Union(union) => {
                composite::union(self, name, union, endian, offset, depth)
            }
            Declaration::Enum(enumeration) => {
                composite::enumeration(self, name, enumeration, endian, offset)
            }
            Declaration::Bitfield(bitfield) => {
                composite::bitfield(self, name, bitfield, endian, offset)
            }
            Declaration::Placement(_) => Err(fail(
                UNKNOWN_TYPE,
                "a placed variable cannot be used as a type",
            )),
        }
    }

    fn scalar(
        &self,
        name: &str,
        builtin: Builtin,
        endian: Endian,
        offset: u64,
    ) -> Result<Node, PatternError> {
        let size = builtin.size();
        let value = match builtin {
            Builtin::Unsigned(_) => {
                NodeValue::Unsigned(self.reader.unsigned(offset, size, endian)?)
            }
            Builtin::Signed(_) => NodeValue::Signed(self.reader.signed(offset, size, endian)?),
            Builtin::Float => {
                let bits = self.reader.unsigned(offset, 4, endian)?;
                NodeValue::Float(f32::from_bits(truncate_u32(bits)))
            }
            Builtin::Double => {
                let bits = self.reader.unsigned(offset, 8, endian)?;
                NodeValue::Double(f64::from_bits(truncate_u64(bits)))
            }
            Builtin::Bool => NodeValue::Bool(self.reader.unsigned(offset, 1, endian)? != 0),
            Builtin::Char => {
                let raw = self.reader.unsigned(offset, 1, endian)?;
                NodeValue::Char(char::from(truncate_u8(raw)))
            }
            Builtin::Char16 => {
                let raw = self.reader.unsigned(offset, 2, endian)?;
                NodeValue::Char(
                    char::from_u32(u32::from(truncate_u16(raw)))
                        .unwrap_or(char::REPLACEMENT_CHARACTER),
                )
            }
        };
        Ok(Node {
            name: name.to_string(),
            type_name: builtin.name().to_string(),
            offset,
            size: u64::from(size),
            value,
        })
    }

    /// Counts one node against the budget so untrusted array lengths cannot
    /// drive unbounded allocation.
    pub(super) fn charge(&mut self) -> Result<(), PatternError> {
        self.nodes += 1;
        if self.nodes > self.options.max_nodes {
            return Err(fail(NODE_BUDGET, "pattern produces too many nodes"));
        }
        Ok(())
    }
}

/// Renders a type reference the way it was written, for display only.
pub(super) fn type_name(type_reference: &TypeReference) -> String {
    match &type_reference.kind {
        TypeKind::Builtin(builtin) => builtin.name().to_string(),
        TypeKind::Named(name) => name.clone(),
    }
}

pub(crate) fn fail(code: &'static str, detail: &'static str) -> PatternError {
    PatternError::new(code, Position { line: 0, column: 0 }, detail)
}

fn truncate_u8(value: u128) -> u8 {
    u8::try_from(value & 0xff).unwrap_or(0)
}

fn truncate_u16(value: u128) -> u16 {
    u16::try_from(value & 0xffff).unwrap_or(0)
}

fn truncate_u32(value: u128) -> u32 {
    u32::try_from(value & 0xffff_ffff).unwrap_or(0)
}

fn truncate_u64(value: u128) -> u64 {
    u64::try_from(value & u128::from(u64::MAX)).unwrap_or(0)
}
