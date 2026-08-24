use crate::ast::Endian;

/// Bounds and defaults applied while evaluating a pattern.
///
/// Array lengths and nesting come from untrusted pattern text, so evaluation
/// is always bounded: it cannot allocate without limit or recurse forever.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvalOptions {
    /// Byte order for types with no explicit `be` / `le` prefix.
    pub endian: Endian,
    /// Maximum number of nodes the value tree may contain.
    pub max_nodes: u64,
    /// Maximum type nesting depth.
    pub max_depth: u32,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            endian: Endian::Little,
            max_nodes: 1_000_000,
            max_depth: 64,
        }
    }
}
