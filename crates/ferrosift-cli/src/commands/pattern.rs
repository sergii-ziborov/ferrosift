//! Pattern validation and evaluation through the native command.

use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

use ferrosift_pattern::{EvalOptions, MapResolver, ResolveLimits};
use serde::Serialize;

use crate::{
    error::CliError,
    io, limits,
    pattern_value::PatternNodeReport,
};

pub fn validate(
    pattern_path: &Path,
    sources: &[String],
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    let _pattern = load_pattern(pattern_path, sources, standard_input)?;
    io::write_line(standard_output, "valid")
}

pub fn run(
    pattern_path: &Path,
    input_path: &Path,
    output_path: &Path,
    sources: &[String],
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    if pattern_path == Path::new("-") && input_path == Path::new("-") {
        return Err(CliError::new(
            "cli.io.stdin_conflict",
            "pattern and input cannot both use standard input",
        ));
    }

    let pattern = load_pattern(pattern_path, sources, standard_input)?;
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
        source_count: pattern.sources.len(),
        node_count: nodes.len(),
        nodes: PatternNodeReport::from_nodes(&nodes),
    };
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| CliError::new("cli.output.serialization", error.to_string()))?;
    io::write_output(output_path, standard_output, &bytes)
}

fn load_pattern(
    path: &Path,
    sources: &[String],
    standard_input: &mut dyn Read,
) -> Result<ferrosift_pattern::Pattern, CliError> {
    let source = read_pattern(path, standard_input)?;
    if sources.is_empty() {
        return ferrosift_pattern::parse(&source).map_err(|error| pattern_error(&error));
    }
    let resolver = build_resolver(sources)?;
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<input>")
        .to_owned();
    ferrosift_pattern::parse_with(label, &source, &resolver, ResolveLimits::default())
        .map_err(|error| pattern_error(&error))
}

fn build_resolver(sources: &[String]) -> Result<MapResolver, CliError> {
    let mut resolver = MapResolver::new();
    for entry in sources {
        let (specifier, path) = entry.split_once('=').ok_or_else(|| {
            CliError::new(
                "cli.pattern.source_malformed",
                "expected specifier=path",
            )
        })?;
        if specifier.is_empty() || path.is_empty() {
            return Err(CliError::new(
                "cli.pattern.source_malformed",
                "expected specifier=path",
            ));
        }
        let bytes = fs::read(path).map_err(|error| {
            CliError::new(
                "cli.pattern.source_read",
                format!("{path}: {error}"),
            )
        })?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits::RECIPE_BYTES {
            return Err(CliError::new(
                "cli.pattern.too_large",
                format!("{path}: limit={}", limits::RECIPE_BYTES),
            ));
        }
        let text = String::from_utf8(bytes)
            .map_err(|error| CliError::new("cli.pattern.invalid_utf8", error.to_string()))?;
        resolver.insert(specifier, text);
    }
    Ok(resolver)
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
    source_count: usize,
    node_count: usize,
    nodes: Vec<PatternNodeReport>,
}
