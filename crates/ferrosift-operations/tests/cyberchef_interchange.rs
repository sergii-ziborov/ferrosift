//! `CyberChef` 11.3 recipe interchange through production operations.

use ferrosift_compat::cyberchef::import_recipe;
use ferrosift_core::{Executor, NeverCancelled};
use ferrosift_model::{CapabilitySet, Value};

mod support;

#[test]
fn imported_base64_recipe_executes_without_fixture_operations() {
    let registry = support::registry();
    let source = br#"[
        {"op":"To Base64","args":["A-Za-z0-9+/="]},
        {"op":"From Base64","args":["A-Za-z0-9+/=",true,false]}
    ]"#;
    let report = import_recipe(source, &registry).expect("bounded recipe must parse");
    let recipe = report.recipe.expect("every operation must map exactly");
    assert!(report.findings.is_empty());

    let input = Value::Bytes(b"FerroSift".to_vec());
    let result = Executor::new(&registry)
        .execute(
            &recipe,
            input.clone(),
            support::budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("imported recipe must execute");
    assert_eq!(result.value, input);
}
