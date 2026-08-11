//! Construction and wire-format tests for the recipe IR.

use ferrosift_model::{
    ArgumentValue, Arguments, OperationId, Recipe, RecipeMetadata, RecipeStep, SchemaVersion,
    StepId,
};

fn step(id: &str) -> RecipeStep {
    let mut arguments = Arguments::new();
    arguments.insert("alphabet".into(), ArgumentValue::Text("standard".into()));

    RecipeStep {
        id: StepId::new(id).expect("valid step ID"),
        operation: OperationId::new("encoding.base64.decode@1").expect("valid operation ID"),
        arguments,
        disabled: true,
        breakpoint: true,
    }
}

#[test]
fn recipe_round_trip_preserves_execution_and_metadata_fields() {
    let recipe = Recipe::new(
        vec![step("decode-1")],
        RecipeMetadata {
            name: Some("Decode sample".into()),
            description: Some("A portable recipe".into()),
        },
    )
    .expect("recipe should be valid");

    let json = serde_json::to_string(&recipe).expect("recipe should serialize");
    let restored: Recipe = serde_json::from_str(&json).expect("recipe should deserialize");

    assert_eq!(restored, recipe);
    assert_eq!(restored.schema_version, SchemaVersion::CURRENT.get());
    assert!(restored.steps[0].disabled);
    assert!(restored.steps[0].breakpoint);
    assert_eq!(restored.metadata.name.as_deref(), Some("Decode sample"));
}

#[test]
fn empty_recipe_is_a_valid_portable_identity() {
    let recipe =
        Recipe::new(Vec::new(), RecipeMetadata::default()).expect("empty recipe should be valid");

    assert!(recipe.steps.is_empty());
    assert_eq!(recipe.schema_version, SchemaVersion::CURRENT.get());
}

#[test]
fn duplicate_step_ids_are_rejected_with_a_stable_code() {
    let error = Recipe::new(
        vec![step("decode-1"), step("decode-1")],
        RecipeMetadata::default(),
    )
    .expect_err("duplicate step IDs should fail");

    assert_eq!(error.code(), "model.recipe.duplicate_step_id");
    assert!(error.to_string().contains("decode-1"));
}

#[test]
fn recipe_deserialization_rejects_duplicate_step_ids() {
    let invalid = Recipe {
        schema_version: SchemaVersion::CURRENT.get(),
        steps: vec![step("decode-1"), step("decode-1")],
        metadata: RecipeMetadata::default(),
    };
    let json = serde_json::to_string(&invalid).expect("invalid recipe should still serialize");
    let error = serde_json::from_str::<Recipe>(&json)
        .expect_err("duplicate step IDs should not deserialize");

    assert!(error.to_string().contains("model.recipe.duplicate_step_id"));
}
