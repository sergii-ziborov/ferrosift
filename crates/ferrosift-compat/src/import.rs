//! Bounded `CyberChef` recipe import.

use alloc::vec::Vec;

use ferrosift_core::OperationRegistry;
use ferrosift_model::{CompatibilityProfile, Recipe, RecipeMetadata};

use crate::{
    error::ImportError,
    finding::CompatibilityFinding,
    profile::MAX_RECIPE_BYTES,
    source::{SourceRecipe, parse_source},
    step::map_step,
};

/// Result of preserving and, when safe, converting a source recipe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportReport {
    /// Executable portable recipe, present only for a complete safe mapping.
    pub recipe: Option<Recipe>,
    /// Complete original source representation.
    pub source: SourceRecipe,
    /// Deterministic compatibility divergences.
    pub findings: Vec<CompatibilityFinding>,
}

/// Imports one `CyberChef` JSON recipe, resolving names in `profile`.
///
/// The two reference versions serialize a recipe identically — the whole
/// `Recipe`, `Operation` and `Ingredient` model is byte-for-byte the same
/// between 11.3 and 11.4 apart from one added argument check — so the profile
/// changes which *names* resolve rather than how the JSON is read. An
/// operation the reference introduced in 11.4 has no 11.3 name, and importing
/// it as 11.3 reports it as unknown rather than guessing.
///
/// # Errors
///
/// Returns a stable [`ImportError`] when the source cannot be bounded or
/// decoded, or when `profile` is not a `CyberChef` release.
pub fn import_recipe(
    bytes: &[u8],
    profile: CompatibilityProfile,
    registry: &OperationRegistry,
) -> Result<ImportReport, ImportError> {
    if !profile.is_cyberchef() {
        return Err(ImportError::UnsupportedProfile);
    }
    if bytes.len() > MAX_RECIPE_BYTES {
        return Err(ImportError::SourceTooLarge);
    }

    let source = parse_source(bytes)?;
    let mut findings = Vec::new();
    let mut steps = Vec::with_capacity(source.steps().len());
    for (index, source_step) in source.steps().iter().enumerate() {
        if let Some(step) = map_step(index, source_step.raw(), profile, registry, &mut findings)? {
            steps.push(step);
        }
    }

    let recipe = if findings.is_empty() {
        Some(
            Recipe::new(steps, RecipeMetadata::default())
                .map_err(|_| ImportError::InvalidRecipe)?,
        )
    } else {
        None
    };
    Ok(ImportReport {
        recipe,
        source,
        findings,
    })
}
