//! Shared scaffolding for the fuzz targets.
//!
//! Every target answers the same question — does this input make the crate do
//! something other than succeed or return an error — so the parts that decide
//! *how* it is asked belong in one place. A target that built its own budget
//! would eventually build a different one, and the difference would show up as
//! a crash nobody could reproduce.

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, TextEncoding, TextValue, Value,
};

/// The ceiling every target runs under.
///
/// Deliberately small. A fuzzer measures inputs per second, and an input that
/// legitimately allocates sixty-four megabytes costs more of that than it
/// returns — the interesting failures are wrong answers and panics, not the
/// budget doing its job. It is not zero either: the paths that refuse an
/// oversized output are themselves worth exercising, and a ceiling this low
/// reaches them often.
#[must_use]
pub fn budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 4,
        max_input_bytes: 1 << 16,
        max_output_bytes: 1 << 18,
        max_expansion_ratio: 256,
        max_branches: 64,
        max_flow_depth: 8,
        max_operation_invocations: 1_024,
        max_total_bytes_processed: 1 << 20,
        max_transient_bytes: 256 * 1024 * 1024,
        max_work_units: 1 << 26,
    }
}

/// A registry, built once per process rather than once per input.
///
/// Building it is a few hundred allocations and it never changes, so doing it
/// per input would spend most of the run measuring the registry.
#[must_use]
pub fn registry() -> &'static OperationRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<OperationRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        ferrosift_operations::default_registry().expect("the built-in registry must validate")
    })
}

/// Runs one operation over `input`, discarding whichever way it went.
///
/// Both outcomes are fine and neither is checked. What is being looked for is
/// the third one: a panic, an overflow the release profile turns into a panic,
/// or a debug assertion. An unknown operation id is skipped rather than
/// asserted, so a target naming an operation outside its feature pack degrades
/// to doing nothing instead of failing every input.
pub fn run(operation: &str, arguments: Arguments, input: Value) {
    let Ok(id) = OperationId::new(operation) else {
        return;
    };
    let Ok(recipe) = Recipe::new(
        vec![RecipeStep {
            id: StepId::new("s").expect("a constant step id is valid"),
            operation: id,
            arguments,
            disabled: false,
            breakpoint: false,
        }],
        RecipeMetadata::default(),
    ) else {
        return;
    };
    let _ = Executor::new(registry()).execute(
        &recipe,
        input,
        budget(),
        &NeverCancelled,
        CapabilitySet::new(),
    );
}

/// Runs one operation with no arguments beyond its declared defaults.
pub fn run_bytes(operation: &str, data: &[u8]) {
    run(operation, Arguments::new(), Value::Bytes(data.to_vec()));
}

/// The input as text, when it is valid UTF-8, or nothing.
///
/// Lossy conversion would be worse than skipping: it feeds the operation bytes
/// no caller could have produced, and a crash on a replacement character is a
/// finding about the harness.
#[must_use]
pub fn as_text(data: &[u8]) -> Option<Value> {
    core::str::from_utf8(data).ok().map(|text| {
        Value::Text(TextValue {
            text: text.to_owned(),
            encoding: TextEncoding::Utf8,
        })
    })
}

/// One text argument.
#[must_use]
pub fn text_argument(name: &str, value: &str) -> Arguments {
    Arguments::from([(name.to_owned(), ArgumentValue::Text(value.to_owned()))])
}

/// A toggleString argument, as the model carries one.
#[must_use]
pub fn toggle(name: &str, option: &str, string: &str) -> (String, ArgumentValue) {
    (
        name.to_owned(),
        ArgumentValue::Map(Arguments::from([
            ("option".to_owned(), ArgumentValue::Text(option.to_owned())),
            ("string".to_owned(), ArgumentValue::Text(string.to_owned())),
        ])),
    )
}

/// Splits an input into a selector byte and the rest.
///
/// Targets that cover a family of operations use the first byte to choose one.
/// Taking it from the input rather than running all of them keeps each
/// execution attributable to one operation, which is what makes a crash
/// reproducible from the file the fuzzer saved.
#[must_use]
pub fn select<'a, T: Copy>(choices: &[T], data: &'a [u8]) -> Option<(T, &'a [u8])> {
    let (first, rest) = data.split_first()?;
    let choice = *choices.get(usize::from(*first) % choices.len())?;
    Some((choice, rest))
}
