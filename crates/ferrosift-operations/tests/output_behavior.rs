//! The generator exemption must be narrow.
//!
//! `OutputBehavior::InputIndependent` waives one check — the expansion ratio —
//! for operations whose output does not come from their input. These tests
//! hold that line from both sides: a generator must run where the ratio would
//! have refused it, and every other operation must still be refused.

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, OutputBehavior, Recipe, RecipeMetadata,
    RecipeStep, StepId, TextEncoding, TextValue, Value,
};

/// A budget whose ratio is tight enough that any real growth trips it.
fn tight_budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 8,
        max_input_bytes: 1 << 20,
        max_output_bytes: 1 << 20,
        // Output may not exceed twice the input. A generator handed an empty
        // input would be capped at two bytes without the declaration.
        max_expansion_ratio: 2,
        max_branches: 16,
        max_flow_depth: 4,
        max_operation_invocations: 1024,
        max_total_bytes_processed: 1 << 22,
    }
}

fn recipe(operation: &str, arguments: &[(&str, ArgumentValue)]) -> Recipe {
    let arguments: Arguments = arguments
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    Recipe::new(
        vec![RecipeStep {
            id: StepId::new("s").expect("step id"),
            operation: OperationId::new(operation).expect("operation id"),
            arguments,
            disabled: false,
            breakpoint: false,
        }],
        RecipeMetadata::default(),
    )
    .expect("recipe")
}

fn run(operation: &str, arguments: &[(&str, ArgumentValue)], input: &str) -> Result<Value, ()> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    Executor::new(&registry)
        .execute(
            &recipe(operation, arguments),
            Value::Text(TextValue {
                text: input.to_owned(),
                encoding: TextEncoding::Utf8,
            }),
            tight_budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map(|outcome| outcome.value)
        .map_err(|_| ())
}

#[test]
fn a_generator_runs_where_the_expansion_ratio_would_have_refused_it() {
    // 3^4 is eighty-one characters from an empty input. Against a ratio of two
    // that is forty times over, which is precisely the refusal this class was
    // added to remove.
    let output = run(
        "text.debruijn@1",
        &[
            ("alphabet_size_k", ArgumentValue::Integer(3)),
            ("key_length_n", ArgumentValue::Integer(4)),
        ],
        "",
    )
    .expect("a generator must not be measured against its input");

    let Value::Text(text) = output else {
        panic!("expected text output");
    };
    assert_eq!(text.text.len(), 81);
}

#[test]
fn an_ordinary_operation_is_still_refused_by_the_expansion_ratio() {
    // Upper-casing does not grow at all, so it must succeed under any ratio.
    // This is the control: it shows the tight budget is not simply refusing
    // everything.
    run(
        "text.case.upper@1",
        &[("scope", ArgumentValue::Text("All".to_owned()))],
        "short",
    )
    .expect("an operation that does not grow stays within the ratio");

    // Get All Casings is exponential and declares nothing, so the ratio still
    // applies to it. Eight characters is 256 lines of nine bytes against an
    // eight-byte input: far past a ratio of two, and it must be refused.
    let refused = run("text.case.all@1", &[], "abcdefgh");
    assert!(
        refused.is_err(),
        "an operation that has not declared itself input-independent must still be measured"
    );
}

#[test]
fn the_default_behavior_is_the_conservative_one() {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let generators: Vec<_> = registry
        .catalog()
        .filter(|spec| matches!(spec.output_behavior, OutputBehavior::InputIndependent))
        .map(|spec| spec.id.as_str().to_owned())
        .collect();

    // Forgetting the declaration must fail closed, so the exemption is opt-in
    // and the list of operations holding it stays short enough to read.
    assert_eq!(
        generators,
        vec!["text.debruijn@1".to_owned()],
        "every operation waiving the expansion ratio should be named here"
    );
}
