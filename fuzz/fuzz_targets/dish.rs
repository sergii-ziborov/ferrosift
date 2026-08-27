//! Value conversion between steps, which every recipe does and no operation
//! owns.
//!
//! The reference carries one value between steps and presents it in whatever
//! type the next operation asks for. That conversion sits between every pair of
//! operations in the catalog, so an input it mishandles is an input that
//! mishandles half the recipes rather than one of them. Two-step recipes are
//! what reach it: the first step produces a value and the second asks for a
//! different representation of it.

#![no_main]

use ferrosift_core::{Executor, NeverCancelled};
use ferrosift_model::{
    Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep, StepId, Value,
};
use libfuzzer_sys::fuzz_target;

/// Operations chosen for the *kind* of value they produce, not for what they
/// compute: bytes, text, a number, a structure, and a list.
const PRODUCERS: &[&str] = &[
    "encoding.hex.encode@1",
    "encoding.base64.encode@1",
    "encoding.hexdump.encode@1",
    "math.sum@1",
    "hash.md5@1",
    "analysis.chi_square@1",
    "core.identity@1",
];

/// Operations chosen for the kind of value they ask for.
const CONSUMERS: &[&str] = &[
    "encoding.hex.decode@1",
    "encoding.base64.decode@1",
    "logic.not@1",
    "text.case.upper@1",
    "hash.sha1@1",
    "core.identity@1",
    "math.sum@1",
];

fuzz_target!(|data: &[u8]| {
    let Some((first, rest)) = ferrosift_fuzz::select(PRODUCERS, data) else {
        return;
    };
    let Some((second, rest)) = ferrosift_fuzz::select(CONSUMERS, rest) else {
        return;
    };

    let steps: Vec<RecipeStep> = [first, second]
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| {
            Some(RecipeStep {
                id: StepId::new(if index == 0 { "a" } else { "b" }).ok()?,
                operation: OperationId::new(*operation).ok()?,
                arguments: Arguments::new(),
                disabled: false,
                breakpoint: false,
            })
        })
        .collect();
    if steps.len() != 2 {
        return;
    }
    let Ok(recipe) = Recipe::new(steps, RecipeMetadata::default()) else {
        return;
    };

    let _ = Executor::new(ferrosift_fuzz::registry()).execute(
        &recipe,
        Value::Bytes(rest.to_vec()),
        ferrosift_fuzz::budget(),
        &NeverCancelled,
        CapabilitySet::new(),
    );
});
