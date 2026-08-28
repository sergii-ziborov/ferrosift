//! `CyberChef` 11.3 public compatibility smoke tests.

use ferrosift_compat::cyberchef::{MAX_RECIPE_BYTES, MAX_RECIPE_STEPS, import_recipe};
use ferrosift_core::OperationRegistry;
use ferrosift_model::CompatibilityProfile;

#[test]
fn empty_recipe_imports_as_executable_identity() {
    let report = import_recipe(
        b"[]",
        CompatibilityProfile::CyberChefV11_3,
        &OperationRegistry::new(),
    )
    .expect("empty recipe is valid");

    let recipe = report.recipe.expect("empty recipe is fully supported");
    assert!(recipe.steps.is_empty());
    assert!(report.source.steps().is_empty());
    assert!(report.findings.is_empty());
    assert_eq!(MAX_RECIPE_BYTES, 1_048_576);
    assert_eq!(MAX_RECIPE_STEPS, 4096);
}
