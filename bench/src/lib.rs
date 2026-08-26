//! Shared setup for the `FerroSift` benchmarks.
//!
//! Every benchmark runs `FerroSift` the way a caller would — through a real
//! engine and recipe — rather than reaching past it to the codec.
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

/// The same draw, folded into printable ASCII.
///
/// For operations that read their input as *text* rather than as bytes. The
/// two ports disagree about which those are: rx-chef's ROT13 converts through
/// a lossy UTF-8 reading where `FerroSift`'s works on bytes, so a random draw
/// makes the two do different work — and a comparison of different work is not
/// a comparison. Letters and digits are what ROT13 is for, and both sides
/// agree on them.
///
/// Digits are included deliberately. The benchmark asks both sides *not* to
/// rotate them, so their presence checks an agreement rather than assuming it.
///
/// The alphabet is sixty-four characters and a byte is drawn from two hundred
/// and fifty-six, so the fold is uniform rather than biased toward its first
/// sixteen entries.
#[must_use]
pub fn sample_text(length: usize) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .";
    sample(length)
        .into_iter()
        .map(|byte| ALPHABET[usize::from(byte) % ALPHABET.len()])
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

/// Compiles a single-step pipeline against an engine.
///
/// This is the path the crate's own documentation points a caller at, and it
/// resolves the operation and its arguments once instead of on every run.
/// Comparing a specialist crate's fast path against `FerroSift`'s slow one
/// would be asymmetric in the other direction — flattering the competitor —
/// so both `FerroSift` entry points are measured and labelled.
///
/// # Panics
///
/// Panics if the pipeline does not compile, which is a benchmark bug.
#[must_use]
pub fn compiled<'a>(
    engine: &'a Engine,
    operation: &str,
    arguments: &[(&str, ArgumentValue)],
) -> ferrosift::CompiledPipeline<'a> {
    let arguments: Arguments = arguments
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    ferrosift::pipeline()
        .budget(budget())
        .step(operation, arguments)
        .compile(engine)
        .expect("benchmark pipeline must compile")
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
