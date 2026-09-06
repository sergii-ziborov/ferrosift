//! Bounded recipe execution and typed or raw output.

use std::{
    io::{Read, Write},
    path::Path,
};

use ferrosift_core::{ExecutionStatus, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::CapabilitySet;

use crate::{
    args::{InputKind, RecipeFormat, ResultFormat},
    error::CliError,
    io, limits, recipe, result, value,
};

pub struct Request<'a> {
    pub format: RecipeFormat,
    pub input_kind: InputKind,
    pub recipe_path: &'a Path,
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub result_format: ResultFormat,
}

pub fn run(
    registry: &OperationRegistry,
    request: &Request<'_>,
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    if request.recipe_path == Path::new("-") && request.input_path == Path::new("-") {
        return Err(CliError::new(
            "cli.io.stdin_conflict",
            "recipe and input cannot both use standard input",
        ));
    }

    let recipe_bytes = io::read_limited(
        request.recipe_path,
        standard_input,
        limits::RECIPE_BYTES,
        "cli.recipe.too_large",
    )?;
    let recipe = recipe::load(&recipe_bytes, request.format, registry)?;
    let input_bytes = io::read_limited(
        request.input_path,
        standard_input,
        limits::INPUT_BYTES,
        "cli.input.too_large",
    )?;
    let input = value::input(input_bytes, request.input_kind)?;
    let execution = Executor::new(registry)
        .execute(
            &recipe,
            input,
            limits::budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|error| CliError::execution(&error))?;

    match request.result_format {
        ResultFormat::Raw => {
            if let ExecutionStatus::Paused { step_index } = execution.status {
                return Err(CliError::new(
                    "cli.execution.paused",
                    format!("step={step_index}"),
                ));
            }
            let bytes = value::output(execution.value)?;
            io::write_output(request.output_path, standard_output, &bytes)
        }
        ResultFormat::Json => {
            let paused_step = match execution.status {
                ExecutionStatus::Paused { step_index } => Some(step_index),
                ExecutionStatus::Completed => None,
            };
            let report = result::ExecutionReport::from_execution(&execution);
            let bytes = serde_json::to_vec_pretty(&report)
                .map_err(|error| CliError::new("cli.output.serialization", error.to_string()))?;
            io::write_output(request.output_path, standard_output, &bytes)?;
            if let Some(step_index) = paused_step {
                return Err(CliError::new(
                    "cli.execution.paused",
                    format!("step={step_index}"),
                ));
            }
            Ok(())
        }
    }
}
