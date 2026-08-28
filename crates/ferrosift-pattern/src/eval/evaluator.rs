use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::composite;
use super::expression::{self, Scope};
use super::options::EvalOptions;
use super::reader::{Reader, out_of_bounds};
use super::source::ByteSource;
use super::value::{Node, NodeValue, ScalarArray, scalar_value};
use crate::ast::{ArrayLength, Builtin, Declaration, Endian, Pattern, TypeKind, TypeReference};
use crate::error::{PatternError, Position};

pub(super) const UNKNOWN_TYPE: &str = "pattern.eval.unknown_type";
pub(super) const DEPTH_EXCEEDED: &str = "pattern.eval.depth_exceeded";
pub(super) const NODE_BUDGET: &str = "pattern.eval.node_budget_exceeded";
pub(super) const INVALID_LENGTH: &str = "pattern.eval.invalid_length";
pub(super) const ZERO_WIDTH_LOOP: &str = "pattern.eval.zero_width_loop";

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
    evaluate_with(pattern, data, options)
}

/// Evaluates every placement in `pattern` against any [`ByteSource`].
///
/// The same evaluation as [`evaluate`], which is this function over a slice.
/// It exists because a slice requires the whole subject in memory, and the
/// case this engine is for — a disk image, a firmware dump, a memory capture —
/// is routinely larger than the machine reading it. A pattern touches a
/// vanishing fraction of what it describes, so the whole is never needed at
/// once; only the scalars a placement actually names.
///
/// # Errors
///
/// As [`evaluate`], plus `pattern.eval.source_failed` when the source declines
/// a read the evaluator had already found to be in range.
pub fn evaluate_with<S: ByteSource + ?Sized>(
    pattern: &Pattern,
    source: &S,
    options: &EvalOptions,
) -> Result<Vec<Node>, PatternError> {
    let mut evaluator = Evaluator {
        pattern,
        reader: Reader::new(source),
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
                pattern: Some(pattern),
            };
            let address = expression::evaluate(&placement.address, scope)?;
            let offset = u64::try_from(address)
                .map_err(|_| out_of_bounds("placement address is past the addressable range"))?;
            let scope = Scope {
                siblings: &roots,
                offset,
                pattern: Some(pattern),
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

pub(super) struct Evaluator<'a, S: ?Sized> {
    pub(super) pattern: &'a Pattern,
    pub(super) reader: Reader<'a, S>,
    pub(super) options: EvalOptions,
    nodes: u64,
}

impl<S: ByteSource + ?Sized> Evaluator<'_, S> {
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
        // An array whose element is a built-in scalar is kept as the bytes it
        // came from rather than as one node per element. See
        // [`NodeValue::Scalars`]; the short version is that a node costs two
        // heap strings and a `u8` costs one byte.
        if let Some((element, endian)) = self.scalar_element(type_reference, depth)? {
            return self.scalar_array(
                name,
                type_reference,
                (element, endian),
                length,
                offset,
                scope,
            );
        }
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
                // reads as.
                //
                // An element of zero width does not advance the cursor, so a
                // condition that reads `$` never changes its mind and the loop
                // runs until something else stops it. The node budget did stop
                // it, at a million wasted iterations and a million allocated
                // nodes -- a bound rather than an answer. Refusing on the first
                // element that occupies nothing costs one comparison and says
                // what is actually wrong.
                let mut index: u64 = 0;
                loop {
                    let test = Scope {
                        siblings: scope.siblings,
                        offset: cursor,
                        pattern: scope.pattern,
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
                    if child.end() == cursor {
                        return Err(fail(
                            ZERO_WIDTH_LOOP,
                            "a while-array element occupies no bytes, so the loop cannot end",
                        ));
                    }
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

    /// The built-in this type reference resolves to, following `using`
    /// aliases, or `None` when it names a composite.
    ///
    /// Aliases are followed rather than refused, because `using Byte = u8;`
    /// then `Byte data[0x100000];` is the same megabyte as `u8 data[…]` and it
    /// would be a strange rule that made one of them a hundred times more
    /// expensive than the other.
    ///
    /// The endianness resolved here is the one each element is read in: the
    /// use site's prefix, else the alias target's, else the default — the same
    /// order [`Evaluator::named`] applies.
    fn scalar_element(
        &self,
        type_reference: &TypeReference,
        depth: u32,
    ) -> Result<Option<(Builtin, Endian)>, PatternError> {
        let mut current = type_reference.clone();
        let mut hops = 0_u32;
        loop {
            match &current.kind {
                TypeKind::Builtin(builtin) => {
                    let endian = current.endian.unwrap_or(self.options.endian);
                    return Ok(Some((*builtin, endian)));
                }
                TypeKind::Named(name) => {
                    // The same ceiling a nested type answers to, so a cycle of
                    // aliases is refused here rather than looping.
                    hops += 1;
                    if depth.saturating_add(hops) > self.options.max_depth {
                        return Err(fail(DEPTH_EXCEEDED, "type nesting is too deep"));
                    }
                    let Some(Declaration::Alias(alias)) = self.pattern.type_named(name) else {
                        return Ok(None);
                    };
                    current = TypeReference {
                        kind: alias.target.kind.clone(),
                        endian: current.endian.or(alias.target.endian),
                    };
                }
            }
        }
    }

    /// Reads a whole array of scalars as one block of bytes.
    fn scalar_array(
        &mut self,
        name: &str,
        type_reference: &TypeReference,
        element: (Builtin, Endian),
        length: &ArrayLength,
        offset: u64,
        scope: Scope<'_>,
    ) -> Result<Node, PatternError> {
        let (element, endian) = element;
        let width = u64::from(element.size());
        let count = match length {
            ArrayLength::Counted(expression) => {
                let count = expression::evaluate(expression, scope)?;
                u64::try_from(count)
                    .map_err(|_| fail(INVALID_LENGTH, "array length is negative or too large"))?
            }
            ArrayLength::While(condition) => {
                // A zero-width element cannot end the loop, exactly as for a
                // composite one — and here it is decidable before the first
                // iteration rather than after it.
                if width == 0 {
                    return Err(fail(
                        ZERO_WIDTH_LOOP,
                        "a while-array element occupies no bytes, so the loop cannot end",
                    ));
                }
                let mut count = 0_u64;
                let mut cursor = offset;
                loop {
                    let test = Scope {
                        siblings: scope.siblings,
                        offset: cursor,
                        pattern: scope.pattern,
                    };
                    if expression::evaluate(condition, test)? == 0 {
                        break;
                    }
                    // The read is charged and bounds-checked per element, so a
                    // condition that never turns false stops at the budget or
                    // at the end of the data rather than running away.
                    self.charge()?;
                    let end = cursor
                        .checked_add(width)
                        .ok_or_else(|| out_of_bounds("read offset overflows"))?;
                    if end > self.reader.len() {
                        return Err(out_of_bounds("read extends past the end of the data"));
                    }
                    cursor = end;
                    count += 1;
                }
                count
            }
        };

        // Charged per element even though no node is built, so the boundary
        // between what evaluates and what is refused does not move: an array
        // this budget used to decline still declines, and one it allowed now
        // costs bytes rather than nodes.
        self.charge_many(count)?;

        let size = count
            .checked_mul(width)
            .ok_or_else(|| fail(INVALID_LENGTH, "array is too large to address"))?;
        let bytes = self.reader.block(offset, size)?;
        Ok(Node {
            name: name.to_string(),
            type_name: format!("{}[{count}]", type_name(type_reference)),
            offset,
            size,
            value: NodeValue::Scalars(ScalarArray::new(element, endian, bytes)),
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
            Declaration::Union(union) => composite::union(self, name, union, endian, offset, depth),
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
        // The same interpretation an array element gets, from the same
        // function, so a scalar written on its own and the same type written
        // with `[n]` after it cannot decode differently.
        let raw = self.reader.unsigned(offset, size, endian)?;
        let value = scalar_value(builtin, raw);
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
        self.charge_many(1)
    }

    /// Counts `count` nodes at once.
    ///
    /// A scalar array builds no nodes and is charged as if it did. The budget
    /// is the caller's statement about how large a value tree they are willing
    /// to receive, and answering `u8 x[10000000]` because it happens to be
    /// cheap now would move that boundary without being asked to.
    fn charge_many(&mut self, count: u64) -> Result<(), PatternError> {
        self.nodes = self.nodes.saturating_add(count);
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
