//! Strict `CyberChef` recipe export.

use alloc::{string::String, vec::Vec};

use ferrosift_core::OperationRegistry;
use ferrosift_model::{CompatibilityProfile, Recipe};
use serde_json::Value as JsonValue;

use crate::{
    arguments::write_arguments, error::ExportError, json_writer::CappedJson,
    profile::MAX_RECIPE_STEPS, source::SourceRecipe,
};

/// Serializes the complete preserved source recipe as compact JSON.
///
/// # Errors
///
/// Returns [`ExportError::Serialization`] if the JSON serializer fails.
pub fn export_source(source: &SourceRecipe) -> Result<String, ExportError> {
    let steps: Vec<&JsonValue> = source.raw_steps().collect();
    serde_json::to_string(&steps).map_err(|_| ExportError::Serialization)
}

/// Exports a portable recipe through exact `CyberChef` 11.3 operation aliases.
///
/// # Errors
///
/// Returns a stable [`ExportError`] when any step cannot be represented exactly.
pub fn export_recipe(recipe: &Recipe, registry: &OperationRegistry) -> Result<String, ExportError> {
    if recipe.steps.len() > MAX_RECIPE_STEPS {
        return Err(ExportError::TooManySteps);
    }

    let mut writer = CappedJson::new();
    writer.push_raw("[")?;
    for (position, step) in recipe.steps.iter().enumerate() {
        if position > 0 {
            writer.push_raw(",")?;
        }
        let operation = registry
            .get(&step.operation)
            .ok_or(ExportError::UnknownOperation)?;
        let mut aliases = operation
            .spec()
            .aliases
            .iter()
            .filter(|alias| alias.profile == CompatibilityProfile::CyberChefV11_3);
        let alias = aliases.next().ok_or(ExportError::MissingAlias)?;
        if aliases.next().is_some() {
            return Err(ExportError::AmbiguousAlias);
        }

        writer.push_raw("{\"op\":")?;
        writer.push_string(&alias.name)?;
        writer.push_raw(",\"args\":")?;
        write_arguments(&mut writer, &step.arguments, &operation.spec().arguments)?;
        if step.disabled {
            writer.push_raw(",\"disabled\":true")?;
        }
        if step.breakpoint {
            writer.push_raw(",\"breakpoint\":true")?;
        }
        writer.push_raw("}")?;
    }
    writer.push_raw("]")?;
    Ok(writer.finish())
}
