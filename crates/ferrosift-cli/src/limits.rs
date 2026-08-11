//! Fixed resource ceilings for the native command.

use ferrosift_compat::cyberchef::{MAX_RECIPE_BYTES, MAX_RECIPE_STEPS};
use ferrosift_core::ExecutionBudget;

pub const RECIPE_BYTES: u64 = MAX_RECIPE_BYTES as u64;
pub const INPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

pub const fn budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: MAX_RECIPE_STEPS,
        max_input_bytes: INPUT_BYTES,
        max_output_bytes: OUTPUT_BYTES,
        max_expansion_ratio: 64,
    }
}
