//! Explicit format-specific recipe loading.

use std::fmt::Write as _;

use ferrosift_compat::cyberchef;
use ferrosift_core::OperationRegistry;
use ferrosift_model::{Recipe, SchemaVersion};

use crate::{args::RecipeFormat, error::CliError};

pub fn load(
    bytes: &[u8],
    format: RecipeFormat,
    registry: &OperationRegistry,
) -> Result<Recipe, CliError> {
    match format {
        RecipeFormat::FerroSift => load_native(bytes),
        RecipeFormat::CyberChefV11_3 => load_cyberchef(bytes, registry),
    }
}

fn load_native(bytes: &[u8]) -> Result<Recipe, CliError> {
    let recipe: Recipe = serde_json::from_slice(bytes)
        .map_err(|error| CliError::new("cli.recipe.malformed", error.to_string()))?;
    if recipe.schema_version != SchemaVersion::CURRENT.get() {
        return Err(CliError::new(
            "cli.recipe.schema_unsupported",
            format!("schema_version={}", recipe.schema_version),
        ));
    }
    Ok(recipe)
}

fn load_cyberchef(bytes: &[u8], registry: &OperationRegistry) -> Result<Recipe, CliError> {
    let report = cyberchef::import_recipe(bytes, registry)
        .map_err(|error| CliError::new(error.code(), error.to_string()))?;
    if let Some(first) = report.findings.first() {
        let mut detail = String::new();
        for (position, finding) in report.findings.iter().enumerate() {
            if position > 0 {
                detail.push_str("; ");
            }
            let _ = write!(detail, "{} step={}", finding.code, finding.source_step);
            if let Some(operation) = &finding.original_operation {
                let _ = write!(detail, " operation={operation}");
            }
        }
        return Err(CliError::new(first.code, detail));
    }
    report.recipe.ok_or_else(|| {
        CliError::new(
            "cli.recipe.not_executable",
            "compatibility import produced no recipe",
        )
    })
}
