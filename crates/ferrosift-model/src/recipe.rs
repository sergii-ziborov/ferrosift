//! Validated, serializable recipe structure.

use alloc::{collections::BTreeSet, string::String, vec::Vec};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::{Arguments, ModelError, OperationId, SchemaVersion, StepId};

/// Human-facing metadata that does not affect recipe execution.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeMetadata {
    /// Optional display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional longer description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One ordered operation invocation inside a recipe.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeStep {
    /// Stable identity used by traces, breakpoints, and editor state.
    pub id: StepId,
    /// Versioned operation contract invoked by this step.
    pub operation: OperationId,
    /// Typed operation arguments.
    pub arguments: Arguments,
    /// Whether execution should preserve the input without invoking the operation.
    #[serde(default)]
    pub disabled: bool,
    /// Whether execution should pause before invoking the operation.
    #[serde(default)]
    pub breakpoint: bool,
}

/// A versioned, ordered, and validated portable recipe.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Recipe {
    /// Serialized model schema version.
    pub schema_version: u32,
    /// Operation steps in execution order.
    pub steps: Vec<RecipeStep>,
    /// Human-facing metadata.
    pub metadata: RecipeMetadata,
}

impl Recipe {
    /// Creates a recipe using the current schema version.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::DuplicateStepId`] when two steps have the same ID.
    pub fn new(steps: Vec<RecipeStep>, metadata: RecipeMetadata) -> Result<Self, ModelError> {
        let recipe = Self {
            schema_version: SchemaVersion::CURRENT.get(),
            steps,
            metadata,
        };
        recipe.validate()?;
        Ok(recipe)
    }

    /// Validates semantic invariants that the type system cannot express.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::DuplicateStepId`] for the first repeated step ID.
    pub fn validate(&self) -> Result<(), ModelError> {
        let mut ids = BTreeSet::new();
        for step in &self.steps {
            if !ids.insert(&step.id) {
                return Err(ModelError::DuplicateStepId {
                    id: String::from(step.id.as_str()),
                });
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct RecipeWire {
    schema_version: u32,
    steps: Vec<RecipeStep>,
    #[serde(default)]
    metadata: RecipeMetadata,
}

impl<'de> Deserialize<'de> for Recipe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RecipeWire::deserialize(deserializer)?;
        let recipe = Self {
            schema_version: wire.schema_version,
            steps: wire.steps,
            metadata: wire.metadata,
        };
        recipe.validate().map_err(D::Error::custom)?;
        Ok(recipe)
    }
}
