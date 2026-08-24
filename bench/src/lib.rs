//! Shared setup for the FerroSift benchmarks.
//!
//! Every benchmark runs FerroSift the way a caller would — through a compiled
//! pipeline on a real engine — rather than reaching past it to the codec.
//! Measuring the codec alone would flatter the library by hiding the argument
//! resolution, budget checks, and value wrapping that a real call pays for,
//! and those are exactly what a comparison against a single-function crate
//! needs to include to be honest.

use ferrosift::{Engine, Error};
use ferrosift_core::{ExecutionBudget, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, Value,
};

/// Deterministic input, so a run is comparable with the one before it.
///
/// A seeded xorshift rather than a clock or `rand`: the same bytes every time
/// means a regression is a real change in the code, not in the data.
#[must_use]
pub fn sample(length: usize) -> Vec<u8> {
    let mut state: u32 = 0x5f37_1d10;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            u8::try_from(state & 0xff).unwrap_or(0)
        })
        .collect()
}

/// The input sizes every benchmark sweeps.
///
/// The small sizes are where per-call overhead dominates and a library with a
/// dispatch layer can lose to a direct function call; the large ones are where
/// the algorithm itself decides. A comparison that reports only one of them is
/// choosing its answer.
pub const SIZES: [usize; 5] = [16, 256, 4096, 65_536, 1_048_576];

/// An engine with the whole portable catalog.
///
/// # Errors
///
/// Returns [`Error`] if the built-in registry does not validate.
pub fn engine() -> Result<Engine, Error> {
    Engine::portable()
}

/// A generous budget: the benchmarks measure the operation, not the limiter.
#[must_use]
pub const fn budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 64,
        max_input_bytes: 1 << 30,
        max_output_bytes: 1 << 30,
        max_expansion_ratio: 1024,
        max_branches: 1024,
        max_flow_depth: 8,
        max_operation_invocations: 1 << 20,
        max_total_bytes_processed: 1 << 34,
    }
}

/// Builds a single-step recipe.
///
/// # Panics
///
/// Panics if the identifier or recipe is not valid, which would be a bug in
/// the benchmark rather than a measurable condition.
#[must_use]
pub fn recipe(operation: &str, arguments: &[(&str, ArgumentValue)]) -> Recipe {
    let arguments: Arguments = arguments
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    Recipe::new(
        vec![RecipeStep {
            id: StepId::new("bench").expect("valid step id"),
            operation: OperationId::new(operation).expect("valid operation id"),
            arguments,
            disabled: false,
            breakpoint: false,
        }],
        RecipeMetadata::default(),
    )
    .expect("valid recipe")
}

/// Runs a prepared recipe once and returns the output bytes.
///
/// # Panics
///
/// Panics if execution fails, which in a benchmark means the setup is wrong.
#[must_use]
pub fn run(engine: &Engine, recipe: &Recipe, input: Value) -> Value {
    ferrosift_core::Executor::new(engine.registry())
        .execute(
            recipe,
            input,
            budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("benchmark recipe must execute")
        .value
}

/// A text argument.
#[must_use]
pub fn text(value: &str) -> ArgumentValue {
    ArgumentValue::Text(value.to_owned())
}

/// An integer argument.
#[must_use]
pub const fn integer(value: i128) -> ArgumentValue {
    ArgumentValue::Integer(value)
}

/// A boolean argument.
#[must_use]
pub const fn boolean(value: bool) -> ArgumentValue {
    ArgumentValue::Boolean(value)
}
