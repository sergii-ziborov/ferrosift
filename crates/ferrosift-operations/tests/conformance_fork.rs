//! Conformance for Fork / Merge map-join flow control.

use ferrosift_core::{ExecutionStatus, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, TextEncoding, Value,
};

mod support;

fn step(id: &str, operation: &str, arguments: Arguments) -> RecipeStep {
    RecipeStep {
        id: StepId::new(id).expect("valid step id"),
        operation: OperationId::new(operation).expect("valid operation id"),
        arguments,
        disabled: false,
        breakpoint: false,
    }
}

fn run_recipe(steps: Vec<RecipeStep>, input: Value) -> Value {
    let registry = support::registry();
    let recipe = Recipe::new(steps, RecipeMetadata::default()).expect("valid recipe");
    let result = Executor::new(&registry)
        .execute(
            &recipe,
            input,
            support::budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("recipe should complete");
    assert_eq!(result.status, ExecutionStatus::Completed);
    result.value
}

#[test]
fn fork_maps_from_base64_per_line_and_merges() {
    let value = run_recipe(
        vec![
            step(
                "fork",
                "flow.fork@1",
                Arguments::from([
                    (
                        "split_delimiter".into(),
                        ArgumentValue::Text("\\n".into()),
                    ),
                    (
                        "merge_delimiter".into(),
                        ArgumentValue::Text("\\n".into()),
                    ),
                    ("ignore_errors".into(), ArgumentValue::Boolean(false)),
                ]),
            ),
            step(
                "b64",
                "encoding.base64.decode@1",
                Arguments::from([
                    (
                        "alphabet".into(),
                        ArgumentValue::Text("A-Za-z0-9+/=".into()),
                    ),
                    ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
                    ("strict".into(), ArgumentValue::Boolean(false)),
                ]),
            ),
            step(
                "merge",
                "flow.merge@1",
                Arguments::from([("merge_all".into(), ArgumentValue::Boolean(true))]),
            ),
        ],
        support::text("aGVsbG8=\nb29y"),
    );
    let Value::Text(text) = value else {
        panic!("expected text");
    };
    assert_eq!(text.text, "hello\noor");
    assert_eq!(text.encoding, TextEncoding::Utf8);
}

#[test]
fn fork_without_body_rejoins_splits() {
    let value = run_recipe(
        vec![
            step(
                "fork",
                "flow.fork@1",
                Arguments::from([
                    (
                        "split_delimiter".into(),
                        ArgumentValue::Text("|".into()),
                    ),
                    (
                        "merge_delimiter".into(),
                        ArgumentValue::Text(",".into()),
                    ),
                    ("ignore_errors".into(), ArgumentValue::Boolean(false)),
                ]),
            ),
            step(
                "merge",
                "flow.merge@1",
                Arguments::from([("merge_all".into(), ArgumentValue::Boolean(true))]),
            ),
        ],
        support::text("a|b|c"),
    );
    let Value::Text(text) = value else {
        panic!("expected text");
    };
    assert_eq!(text.text, "a,b,c");
}

#[test]
fn fork_ignore_errors_keeps_empty_branch_slot() {
    let value = run_recipe(
        vec![
            step(
                "fork",
                "flow.fork@1",
                Arguments::from([
                    (
                        "split_delimiter".into(),
                        ArgumentValue::Text("\\n".into()),
                    ),
                    (
                        "merge_delimiter".into(),
                        ArgumentValue::Text("\\n".into()),
                    ),
                    ("ignore_errors".into(), ArgumentValue::Boolean(true)),
                ]),
            ),
            step(
                "b64",
                "encoding.base64.decode@1",
                Arguments::from([
                    (
                        "alphabet".into(),
                        ArgumentValue::Text("A-Za-z0-9+/=".into()),
                    ),
                    // Reject noise so an invalid line fails.
                    ("remove_non_alphabet".into(), ArgumentValue::Boolean(false)),
                    ("strict".into(), ArgumentValue::Boolean(true)),
                ]),
            ),
            step(
                "merge",
                "flow.merge@1",
                Arguments::from([("merge_all".into(), ArgumentValue::Boolean(true))]),
            ),
        ],
        support::text("aGVsbG8=\n!!!not-base64!!!"),
    );
    let Value::Text(text) = value else {
        panic!("expected text");
    };
    // First branch decodes; second fails and becomes empty when ignore_errors.
    assert_eq!(text.text, "hello\n");
}

#[test]
fn cyberchef_aliases_resolve_for_fork_and_merge() {
    let registry = support::registry();
    let fork = registry
        .resolve_alias(
            ferrosift_model::CompatibilityProfile::CyberChefV11_3,
            "Fork",
        )
        .expect("Fork alias");
    let merge = registry
        .resolve_alias(
            ferrosift_model::CompatibilityProfile::CyberChefV11_3,
            "Merge",
        )
        .expect("Merge alias");
    assert_eq!(fork.spec().id.as_str(), "flow.fork@1");
    assert_eq!(merge.spec().id.as_str(), "flow.merge@1");
}
