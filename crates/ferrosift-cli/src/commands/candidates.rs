//! Evaluate an explicit candidate-recipe batch against one input.

use std::{
    io::{Read, Write},
    path::Path,
};

use ferrosift_host::{
    CandidateCheck, CandidateRecipe, CandidatesRequest, HostConfig, HostService,
    InputKind as HostInputKind, OpenRequest,
};
use serde::Deserialize;

use crate::{
    args::InputKind,
    error::CliError,
    io, limits,
};

#[derive(Debug, Deserialize)]
struct CandidatesFile {
    candidates: Vec<CandidateFileEntry>,
}

#[derive(Debug, Deserialize)]
struct CandidateFileEntry {
    id: String,
    format: String,
    #[serde(alias = "recipe_json")]
    recipe: serde_json::Value,
    #[serde(default)]
    checks: Vec<CandidateCheck>,
}

/// Runs `candidates` and writes a `ferrosift.candidates.v1` table.
pub fn run(
    input_kind: InputKind,
    input_path: &Path,
    candidates_path: &Path,
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    if input_path == Path::new("-") && candidates_path == Path::new("-") {
        return Err(CliError::new(
            "cli.io.stdin_conflict",
            "input and candidates cannot both use standard input",
        ));
    }

    let input = io::read_limited(
        input_path,
        standard_input,
        limits::INPUT_BYTES,
        "cli.input.too_large",
    )?;
    let document = io::read_limited(
        candidates_path,
        standard_input,
        limits::RECIPE_BYTES,
        "cli.candidates.too_large",
    )?;
    let file: CandidatesFile = serde_json::from_slice(&document)
        .map_err(|error| CliError::new("cli.candidates.malformed", error.to_string()))?;

    let mut recipes = Vec::with_capacity(file.candidates.len());
    for entry in file.candidates {
        let recipe_json = match entry.recipe {
            serde_json::Value::String(text) => text,
            other => serde_json::to_string(&other)
                .map_err(|error| CliError::new("cli.candidates.malformed", error.to_string()))?,
        };
        recipes.push(CandidateRecipe {
            id: entry.id,
            format: entry.format,
            recipe_json,
            checks: entry.checks,
        });
    }

    let host = HostService::new(HostConfig::default())
        .map_err(|error| CliError::new(error.code(), error.detail()))?;
    let opened = host
        .open(OpenRequest::Bytes {
            bytes: input,
            kind: match input_kind {
                InputKind::Bytes => HostInputKind::Bytes,
                InputKind::Text => HostInputKind::Text,
            },
        })
        .map_err(|error| CliError::new(error.code(), error.detail()))?;
    let report = host
        .evaluate_candidates(&CandidatesRequest {
            input_artifact_id: opened.id.as_str(),
            candidates: &recipes,
        })
        .map_err(|error| CliError::new(error.code(), error.detail()))?;

    let body = serde_json::to_vec_pretty(&report)
        .map_err(|error| CliError::new("cli.candidates.serialize", error.to_string()))?;
    standard_output
        .write_all(&body)
        .map_err(CliError::write)?;
    standard_output.write_all(b"\n").map_err(CliError::write)?;
    Ok(())
}
