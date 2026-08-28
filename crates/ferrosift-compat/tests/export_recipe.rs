//! Strict native recipe export tests.

mod support;

use ferrosift_compat::cyberchef::{
    ExportError, MAX_ARGUMENT_DEPTH, MAX_RECIPE_BYTES, MAX_RECIPE_STEPS, export_recipe,
    import_recipe,
};
use ferrosift_model::{
    ArgumentKind, ArgumentValue, Arguments, CompatibilityProfile, OperationId, Recipe,
    RecipeMetadata, RecipeStep, StepId,
};
use serde_json::Value as JsonValue;

use support::{argument, registry_with};

#[test]
fn native_recipe_exports_positional_arguments_and_true_flags() {
    let registry = registry_with(
        "test.transform@1",
        &["Test Transform"],
        vec![
            argument("enabled", ArgumentKind::Boolean, true, None),
            argument(
                "count",
                ArgumentKind::Integer,
                false,
                Some(ArgumentValue::Integer(9)),
            ),
        ],
    );
    let recipe = recipe_with(
        "test.transform@1",
        Arguments::from([("enabled".into(), ArgumentValue::Boolean(true))]),
        true,
        true,
    );

    let json = export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("recipe is exactly representable");
    let value: JsonValue = serde_json::from_str(&json).expect("export is JSON");

    assert_eq!(
        value,
        serde_json::json!([{
            "op": "Test Transform",
            "args": [true, 9],
            "disabled": true,
            "breakpoint": true
        }])
    );
}

#[test]
fn false_flags_are_omitted() {
    let registry = registry_with("test.empty@1", &["Empty"], vec![]);
    let recipe = recipe_with("test.empty@1", Arguments::new(), false, false);

    let json = export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("empty operation is representable");
    let value: JsonValue = serde_json::from_str(&json).expect("export is JSON");

    assert_eq!(value, serde_json::json!([{"op": "Empty", "args": []}]));
}

#[test]
fn missing_and_ambiguous_profile_aliases_fail_distinctly() {
    let no_alias = registry_with("test.empty@1", &[], vec![]);
    let two_aliases = registry_with("test.empty@1", &["Empty", "Also Empty"], vec![]);
    let recipe = recipe_with("test.empty@1", Arguments::new(), false, false);

    assert_eq!(
        export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &no_alias),
        Err(ExportError::MissingAlias)
    );
    assert_eq!(
        export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &two_aliases),
        Err(ExportError::AmbiguousAlias)
    );
}

#[test]
fn unknown_operations_and_undeclared_arguments_fail_closed() {
    let empty_registry = ferrosift_core::OperationRegistry::new();
    let registry = registry_with("test.empty@1", &["Empty"], vec![]);
    let unknown = recipe_with("test.unknown@1", Arguments::new(), false, false);
    let undeclared = recipe_with(
        "test.empty@1",
        Arguments::from([("surprise".into(), ArgumentValue::Boolean(true))]),
        false,
        false,
    );

    assert_eq!(
        export_recipe(
            &unknown,
            CompatibilityProfile::CyberChefV11_3,
            &empty_registry
        ),
        Err(ExportError::UnknownOperation)
    );
    assert_eq!(
        export_recipe(&undeclared, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::UndeclaredArgument)
    );
}

#[test]
fn missing_required_and_unrepresentable_integer_fail_closed() {
    let registry = registry_with(
        "test.number@1",
        &["Number"],
        vec![argument("number", ArgumentKind::Integer, true, None)],
    );
    let missing = recipe_with("test.number@1", Arguments::new(), false, false);
    let huge = recipe_with(
        "test.number@1",
        Arguments::from([("number".into(), ArgumentValue::Integer(i128::MAX))]),
        false,
        false,
    );

    assert_eq!(
        export_recipe(&missing, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::MissingArgument)
    );
    assert_eq!(
        export_recipe(&huge, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::ArgumentValue)
    );
}

#[test]
fn supported_native_export_imports_back_to_equivalent_step_semantics() {
    let registry = registry_with(
        "test.bytes@1",
        &["Bytes"],
        vec![argument("bytes", ArgumentKind::Bytes, true, None)],
    );
    let original = recipe_with(
        "test.bytes@1",
        Arguments::from([("bytes".into(), ArgumentValue::Bytes(vec![0, 127, 255]))]),
        false,
        true,
    );

    let json = export_recipe(&original, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("native export succeeds");
    let imported = import_recipe(
        json.as_bytes(),
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("exported JSON imports")
    .recipe
    .expect("exported semantics are supported");

    assert_eq!(imported.steps[0].operation, original.steps[0].operation);
    assert_eq!(imported.steps[0].arguments, original.steps[0].arguments);
    assert_eq!(imported.steps[0].disabled, original.steps[0].disabled);
    assert_eq!(imported.steps[0].breakpoint, original.steps[0].breakpoint);
}

#[test]
fn native_export_enforces_step_and_serialized_byte_limits() {
    let empty_registry = ferrosift_core::OperationRegistry::new();
    let too_many = recipe_with_steps(MAX_RECIPE_STEPS + 1, "test.unknown@1");
    assert_eq!(
        export_recipe(
            &too_many,
            CompatibilityProfile::CyberChefV11_3,
            &empty_registry
        ),
        Err(ExportError::TooManySteps)
    );

    let registry = registry_with(
        "test.text@1",
        &["Text"],
        vec![argument("text", ArgumentKind::Text, true, None)],
    );
    let oversized = recipe_with(
        "test.text@1",
        Arguments::from([(
            "text".into(),
            ArgumentValue::Text("x".repeat(MAX_RECIPE_BYTES)),
        )]),
        false,
        false,
    );
    assert_eq!(
        export_recipe(&oversized, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::RecipeTooLarge)
    );
}

#[test]
fn native_export_rejects_javascript_unsafe_integers_recursively() {
    let registry = registry_with(
        "test.map@1",
        &["Map"],
        vec![argument("value", ArgumentKind::Map, true, None)],
    );
    let recipe = recipe_with(
        "test.map@1",
        Arguments::from([(
            "value".into(),
            ArgumentValue::Map(
                [(
                    "nested".into(),
                    ArgumentValue::List(vec![ArgumentValue::Integer(-9_007_199_254_740_992)]),
                )]
                .into(),
            ),
        )]),
        false,
        false,
    );

    assert_eq!(
        export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::ArgumentValue)
    );
}

#[test]
fn native_export_rejects_excessively_nested_argument_values() {
    let registry = registry_with(
        "test.list@1",
        &["List"],
        vec![argument("value", ArgumentKind::List, true, None)],
    );
    let nested = nested_list(MAX_ARGUMENT_DEPTH + 1);
    let recipe = recipe_with(
        "test.list@1",
        Arguments::from([("value".into(), nested)]),
        false,
        false,
    );

    assert_eq!(
        export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::ArgumentValue)
    );
}

#[test]
fn native_depth_limit_round_trips_at_exact_boundary() {
    let registry = registry_with(
        "test.list@1",
        &["List"],
        vec![argument("value", ArgumentKind::List, true, None)],
    );
    let nested = nested_list(MAX_ARGUMENT_DEPTH);
    let recipe = recipe_with(
        "test.list@1",
        Arguments::from([("value".into(), nested.clone())]),
        false,
        false,
    );

    let json = export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("boundary depth exports");
    let imported = import_recipe(
        json.as_bytes(),
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("exported boundary depth parses")
    .recipe
    .expect("exported boundary depth remains executable");

    assert_eq!(imported.steps[0].arguments.get("value"), Some(&nested));
}

fn nested_list(depth: usize) -> ArgumentValue {
    (0..depth).fold(ArgumentValue::Boolean(true), |value, _| {
        ArgumentValue::List(vec![value])
    })
}

fn recipe_with(operation: &str, arguments: Arguments, disabled: bool, breakpoint: bool) -> Recipe {
    Recipe::new(
        vec![RecipeStep {
            id: StepId::new("native-0").expect("valid step ID"),
            operation: OperationId::new(operation).expect("valid operation ID"),
            arguments,
            disabled,
            breakpoint,
        }],
        RecipeMetadata::default(),
    )
    .expect("single-step fixture is valid")
}

fn recipe_with_steps(count: usize, operation: &str) -> Recipe {
    let operation = OperationId::new(operation).expect("valid operation ID");
    let steps = (0..count)
        .map(|index| RecipeStep {
            id: StepId::new(format!("native-{index}")).expect("valid step ID"),
            operation: operation.clone(),
            arguments: Arguments::new(),
            disabled: false,
            breakpoint: false,
        })
        .collect();
    Recipe::new(steps, RecipeMetadata::default()).expect("unique step IDs are valid")
}
