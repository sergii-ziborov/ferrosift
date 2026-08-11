//! Bounded `CyberChef` recipe import.

use alloc::vec::Vec;

use ferrosift_core::OperationRegistry;
use ferrosift_model::{Recipe, RecipeMetadata};

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

/// Imports one `CyberChef` 11.3.0 JSON recipe.
///
/// # Errors
///
/// Returns a stable [`ImportError`] when the source cannot be bounded or decoded.
pub fn import_recipe(
    bytes: &[u8],
    registry: &OperationRegistry,
) -> Result<ImportReport, ImportError> {
    if bytes.len() > MAX_RECIPE_BYTES {
        return Err(ImportError::SourceTooLarge);
    }

    let source = parse_source(bytes)?;
    let mut findings = Vec::new();
    let mut steps = Vec::with_capacity(source.steps().len());
    for (index, source_step) in source.steps().iter().enumerate() {
        if let Some(step) = map_step(index, source_step.raw(), registry, &mut findings)? {
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
