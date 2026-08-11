//! Full recipe preflight without operation invocation.

use std::{
    io::{Read, Write},
    path::Path,
};

use ferrosift_core::{Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::CapabilitySet;

use crate::{
    args::{InputKind, RecipeFormat},
    error::CliError,
    io, limits, recipe, value,
};

pub fn run(
    registry: &OperationRegistry,
    format: RecipeFormat,
    input_kind: InputKind,
    recipe_path: &Path,
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), CliError> {
    let bytes = io::read_limited(
        recipe_path,
        input,
        limits::RECIPE_BYTES,
        "cli.recipe.too_large",
    )?;
    let recipe = recipe::load(&bytes, format, registry)?;
    Executor::new(registry)
        .validate(
            &recipe,
            &value::empty(input_kind),
            limits::budget(),
            &NeverCancelled,
            &CapabilitySet::new(),
        )
        .map_err(|error| CliError::execution(&error))?;
    io::write_line(output, "valid")
}
