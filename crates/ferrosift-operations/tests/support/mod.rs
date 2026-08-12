#![allow(dead_code)]

use ferrosift_core::{
    ExecutionBudget, ExecutionResult, Executor, NeverCancelled, OperationRegistry,
};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, Value,
};

pub fn registry() -> OperationRegistry {
    ferrosift_operations::default_registry().expect("built-in registry must validate")
}

pub fn argument(name: &str, value: ArgumentValue) -> Arguments {
    Arguments::from([(name.into(), value)])
}

pub fn run(operation: &str, arguments: Arguments, input: Value) -> ExecutionResult {
    run_with_budget(operation, arguments, input, budget()).expect("operation should succeed")
}

pub fn run_with_budget(
    operation: &str,
    arguments: Arguments,
    input: Value,
    budget: ExecutionBudget,
) -> Result<ExecutionResult, ferrosift_core::ExecutionError> {
    let registry = registry();
    let recipe = Recipe::new(
        vec![RecipeStep {
            id: StepId::new("operation").expect("valid step ID"),
            operation: OperationId::new(operation).expect("valid operation ID"),
            arguments,
            disabled: false,
            breakpoint: false,
        }],
        RecipeMetadata::default(),
    )
    .expect("valid recipe");
    Executor::new(&registry).execute(
        &recipe,
        input,
        budget,
        &NeverCancelled,
        CapabilitySet::new(),
    )
}

pub const fn budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 16,
        max_input_bytes: 1_048_576,
        max_output_bytes: 2_097_152,
        max_expansion_ratio: 64,
    }
}

pub fn text(value: &str) -> Value {
    Value::Text(ferrosift_model::TextValue {
        text: value.into(),
        encoding: ferrosift_model::TextEncoding::Utf8,
    })
}

pub fn output_text(result: ExecutionResult) -> String {
    let Value::Text(value) = result.value else {
        panic!("expected text output")
    };
    value.text
}

pub fn output_bytes(result: ExecutionResult) -> Vec<u8> {
    let Value::Bytes(value) = result.value else {
        panic!("expected byte output")
    };
    value
}
