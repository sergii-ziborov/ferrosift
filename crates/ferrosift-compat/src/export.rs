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

/// Exports a portable recipe through the exact operation names `profile` uses.
///
/// The JSON shape is the same for every `CyberChef` version this speaks; the
/// profile decides which name each operation is written under, and whether it
/// has one at all. An operation the reference introduced in 11.4 cannot be
/// exported as 11.3 — there is no name to write, and
/// [`ExportError::MissingAlias`] says so rather than emitting a recipe the
/// older reference would refuse to load.
///
/// # Errors
///
/// Returns a stable [`ExportError`] when any step cannot be represented
/// exactly, or when `profile` is not a `CyberChef` release.
pub fn export_recipe(
    recipe: &Recipe,
    profile: CompatibilityProfile,
    registry: &OperationRegistry,
) -> Result<String, ExportError> {
    if !profile.is_cyberchef() {
        return Err(ExportError::UnsupportedProfile);
    }
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
            .filter(|alias| alias.profile == profile);
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
