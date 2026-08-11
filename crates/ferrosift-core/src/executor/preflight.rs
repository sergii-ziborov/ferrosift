//! Complete fail-closed recipe preparation before the first invocation.

use alloc::vec::Vec;

use ferrosift_model::{
    Arguments, CapabilitySet, Recipe, RecipeStep, Value, ValueConstraint, ValueKind,
};

use crate::{Cancellation, ExecutionBudget, Operation, OperationError, OperationRegistry};

use super::{ExecutionError, ExecutionFailure, limits, step_location};

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
    let mut prepared = Vec::with_capacity(recipe.steps.len());
    let mut previous_output = ValueConstraint::Exact(input.kind());
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
        if !output_satisfies_input(&previous_output, &operation.spec().input) {
            return Err(ExecutionError::at_step(
                ExecutionFailure::InputKindMismatch,
                step_location(index, step),
                crate::ExecutionTrace::default(),
            ));
        }
        previous_output = operation.spec().output.clone();
        prepared.push(PreparedStep {
            step,
            operation: Some(operation),
            arguments,
        });
    }
    if cancellation.is_cancelled() {
        return Err(ExecutionError::global(ExecutionFailure::Operation(
            OperationError::Cancelled,
        )));
    }
    Ok(prepared)
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
