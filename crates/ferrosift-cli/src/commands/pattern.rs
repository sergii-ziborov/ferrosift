//! Pattern validation and evaluation through the native command.

use std::{
    io::{Read, Write},
    path::Path,
};

use ferrosift_pattern::EvalOptions;
use serde::Serialize;

use crate::{
    error::CliError,
    io, limits,
    pattern_value::PatternNodeReport,
};

pub fn validate(
    pattern_path: &Path,
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    let source = read_pattern(pattern_path, standard_input)?;
    ferrosift_pattern::parse(&source).map_err(|error| pattern_error(&error))?;
    io::write_line(standard_output, "valid")
}

pub fn run(
    pattern_path: &Path,
    input_path: &Path,
    output_path: &Path,
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    if pattern_path == Path::new("-") && input_path == Path::new("-") {
        return Err(CliError::new(
            "cli.io.stdin_conflict",
            "pattern and input cannot both use standard input",
        ));
    }

    let source = read_pattern(pattern_path, standard_input)?;
    let pattern = ferrosift_pattern::parse(&source).map_err(|error| pattern_error(&error))?;
    let input = io::read_limited(
        input_path,
        standard_input,
        limits::INPUT_BYTES,
        "cli.input.too_large",
    )?;
    let nodes = ferrosift_pattern::evaluate(&pattern, &input, &EvalOptions::default())
        .map_err(|error| pattern_error(&error))?;

    let report = PatternRunReport {
        schema: "ferrosift.pattern.v1",
        runtime_version: env!("CARGO_PKG_VERSION"),
        node_count: nodes.len(),
        nodes: PatternNodeReport::from_nodes(&nodes),
    };
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| CliError::new("cli.output.serialization", error.to_string()))?;
    io::write_output(output_path, standard_output, &bytes)
}

fn read_pattern(path: &Path, standard_input: &mut dyn Read) -> Result<String, CliError> {
    let bytes = io::read_limited(path, standard_input, limits::RECIPE_BYTES, "cli.pattern.too_large")?;
    String::from_utf8(bytes)
        .map_err(|error| CliError::new("cli.pattern.invalid_utf8", error.to_string()))
}

fn pattern_error(error: &ferrosift_pattern::PatternError) -> CliError {
    CliError::new(error.code(), error.to_string())
}

#[derive(Serialize)]
struct PatternRunReport {
    schema: &'static str,
    runtime_version: &'static str,
    node_count: usize,
    nodes: Vec<PatternNodeReport>,
}
