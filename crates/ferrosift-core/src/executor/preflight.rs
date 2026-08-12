//! Complete fail-closed recipe preparation before the first invocation.

use alloc::vec::Vec;

use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, Recipe, RecipeStep, Value, ValueConstraint, ValueKind,
};

use crate::{Cancellation, ExecutionBudget, Operation, OperationError, OperationRegistry};

use super::{
    ExecutionError, ExecutionFailure, flow, limits, step_location,
};

pub(super) struct PreparedStep<'recipe, 'registry> {
    pub(super) step: &'recipe RecipeStep,
    pub(super) operation: Option<&'registry dyn Operation>,
    pub(super) arguments: Arguments,
}

pub(super) fn prepare<'recipe, 'registry>(
    recipe: &'recipe Recipe,
    registry: &'registry OperationRegistry,
    input: &Value,
    budget: ExecutionBudget,
    cancellation: &dyn Cancellation,
    capabilities: &CapabilitySet,
) -> Result<Vec<PreparedStep<'recipe, 'registry>>, ExecutionError> {
    recipe
        .validate()
        .map_err(|error| ExecutionError::global(ExecutionFailure::InvalidRecipe(error)))?;
    limits::check_initial(
        recipe.steps.len(),
        crate::ValueSummary::from_value(input).size_bytes,
        budget,
    )
    .map_err(ExecutionError::global)?;

    let prepared = resolve_steps(recipe, registry, capabilities)?;
    check_type_flow(&prepared, input.kind())?;

    if cancellation.is_cancelled() {
        return Err(ExecutionError::global(ExecutionFailure::Operation(
            OperationError::Cancelled,
        )));
    }
    Ok(prepared)
}

fn resolve_steps<'recipe, 'registry>(
    recipe: &'recipe Recipe,
    registry: &'registry OperationRegistry,
    capabilities: &CapabilitySet,
) -> Result<Vec<PreparedStep<'recipe, 'registry>>, ExecutionError> {
    let mut prepared = Vec::with_capacity(recipe.steps.len());
    for (index, step) in recipe.steps.iter().enumerate() {
        if step.disabled {
            prepared.push(PreparedStep {
                step,
                operation: None,
                arguments: Arguments::new(),
            });
            continue;
        }
        let location = step_location(index, step);
        let Some(operation) = registry.get(&step.operation) else {
            return Err(ExecutionError::at_step(
                ExecutionFailure::UnknownOperation,
                location,
                crate::ExecutionTrace::default(),
            ));
        };
        if let Some(capability) = operation
            .spec()
            .capabilities
            .iter()
            .find(|capability| !capabilities.contains(capability))
        {
            return Err(ExecutionError::at_step(
                ExecutionFailure::CapabilityDenied {
                    capability: *capability,
                },
                location,
                crate::ExecutionTrace::default(),
            ));
        }
        let arguments = resolve_arguments(step, operation).map_err(|failure| {
            ExecutionError::at_step(failure, location, crate::ExecutionTrace::default())
        })?;
        prepared.push(PreparedStep {
            step,
            operation: Some(operation),
            arguments,
        });
    }
    Ok(prepared)
}

fn check_type_flow(
    prepared: &[PreparedStep<'_, '_>],
    input_kind: ValueKind,
) -> Result<(), ExecutionError> {
    let mut previous_output = ValueConstraint::Exact(input_kind);
    let mut index = 0;
    while index < prepared.len() {
        if prepared[index].step.disabled {
            index += 1;
            continue;
        }
        let operation = prepared[index]
            .operation
            .expect("enabled steps are resolved");
        let location = step_location(index, prepared[index].step);

        if flow::is_fork(&prepared[index].step.operation) {
            if !output_satisfies_input(&previous_output, &operation.spec().input) {
                return Err(ExecutionError::at_step(
                    ExecutionFailure::InputKindMismatch,
                    location,
                    crate::ExecutionTrace::default(),
                ));
            }
            let merge_index =
                find_merge_for_prepared(index, prepared).unwrap_or(prepared.len());
            validate_fork_body(prepared, index + 1, merge_index)?;
            previous_output = ValueConstraint::Exact(ValueKind::Text);
            index = if merge_index < prepared.len() {
                merge_index + 1
            } else {
                merge_index
            };
            continue;
        }

        if flow::is_merge(&prepared[index].step.operation) {
            index += 1;
            continue;
        }

        if !output_satisfies_input(&previous_output, &operation.spec().input) {
            return Err(ExecutionError::at_step(
                ExecutionFailure::InputKindMismatch,
                location,
                crate::ExecutionTrace::default(),
            ));
        }
        previous_output = operation.spec().output.clone();
        index += 1;
    }
    Ok(())
}

fn validate_fork_body(
    prepared: &[PreparedStep<'_, '_>],
    body_start: usize,
    merge_index: usize,
) -> Result<(), ExecutionError> {
    let mut body_prev = ValueConstraint::Exact(ValueKind::Text);
    for (body_index, step) in prepared
        .iter()
        .enumerate()
        .take(merge_index)
        .skip(body_start)
    {
        if step.step.disabled {
            continue;
        }
        let body_op = step.operation.expect("enabled body steps are resolved");
        if !output_satisfies_input(&body_prev, &body_op.spec().input) {
            return Err(ExecutionError::at_step(
                ExecutionFailure::InputKindMismatch,
                step_location(body_index, step.step),
                crate::ExecutionTrace::default(),
            ));
        }
        body_prev = body_op.spec().output.clone();
    }
    Ok(())
}

fn find_merge_for_prepared(fork_index: usize, prepared: &[PreparedStep<'_, '_>]) -> Option<usize> {
    let ids: Vec<_> = prepared
        .iter()
        .map(|step| step.step.operation.clone())
        .collect();
    let merge_all: Vec<bool> = prepared
        .iter()
        .map(|step| match step.arguments.get("merge_all") {
            Some(ArgumentValue::Boolean(value)) => *value,
            _ => true,
        })
        .collect();
    let disabled: Vec<bool> = prepared.iter().map(|step| step.step.disabled).collect();
    flow::find_merge_index(fork_index, &ids, &merge_all, &disabled)
}

fn output_satisfies_input(output: &ValueConstraint, input: &ValueConstraint) -> bool {
    const ALL_VALUE_KINDS: [ValueKind; 7] = [
        ValueKind::Empty,
        ValueKind::Bytes,
        ValueKind::Text,
        ValueKind::Boolean,
        ValueKind::Integer,
        ValueKind::Structured,
        ValueKind::Files,
    ];

    ALL_VALUE_KINDS
        .iter()
        .copied()
        .all(|kind| !output.accepts(kind) || input.accepts(kind))
}

fn resolve_arguments(
    step: &RecipeStep,
    operation: &dyn Operation,
) -> Result<Arguments, ExecutionFailure> {
    let specifications = &operation.spec().arguments;
    if step
        .arguments
        .keys()
        .any(|name| !specifications.iter().any(|argument| argument.name == *name))
    {
        return Err(invalid_arguments());
    }

    let mut resolved = Arguments::new();
    for specification in specifications {
        if let Some(value) = step.arguments.get(&specification.name) {
            if !specification.kind.matches(value) {
                return Err(invalid_arguments());
            }
            resolved.insert(specification.name.clone(), value.clone());
        } else if let Some(default) = &specification.default {
            resolved.insert(specification.name.clone(), default.clone());
        } else if specification.required {
            return Err(invalid_arguments());
        }
    }
    Ok(resolved)
}

fn invalid_arguments() -> ExecutionFailure {
    ExecutionFailure::Operation(OperationError::InvalidArguments)
}
