//! `CyberChef` 11.3 compatibility limits.

/// Maximum accepted serialized `CyberChef` recipe size.
pub const MAX_RECIPE_BYTES: usize = 1_048_576;

/// Maximum accepted number of `CyberChef` recipe steps.
pub const MAX_RECIPE_STEPS: usize = 4096;

/// Maximum nested list/map depth for executable arguments.
pub const MAX_ARGUMENT_DEPTH: usize = 120;

/// Largest integer guaranteed to round-trip exactly through JavaScript `Number`.
pub const MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;
