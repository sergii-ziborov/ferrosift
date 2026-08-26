//! Complete fail-closed recipe preparation before the first invocation.
//!
//! Preparation splits in two. [`resolve`] does the work that depends only on
//! the recipe, the registry, and the granted capabilities: structural
//! validation, operation lookup, capability checks, and argument resolution.
//! [`check_runtime`] does what genuinely depends on the call: the input size
//! and representation, the budget, and cancellation.
//!
//! That split is what lets a prepared recipe be reused: the per-step lookups
//! and argument work happen once, not on every run.

use alloc::vec::Vec;

use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeStep, StepId, Value,
    ValueConstraint, ValueKind,
};

use crate::{Cancellation, ExecutionBudget, Operation, OperationError, OperationRegistry};

use super::{ExecutionError, ExecutionFailure, StepLocation, flow, limits};

/// One recipe step with everything resolvable already resolved.
///
/// The step's own identity is owned rather than borrowed so a prepared recipe
/// outlives the [`Recipe`] it came from and can be stored alongside it.
pub(super) struct PreparedStep<'registry> {
    pub(super) id: StepId,
    pub(super) operation_id: OperationId,
    pub(super) disabled: bool,
    pub(super) breakpoint: bool,
    pub(super) operation: Option<&'registry dyn Operation>,
    pub(super) arguments: Arguments,
}

impl PreparedStep<'_> {
    pub(super) fn location(&self, index: usize) -> StepLocation {
        StepLocation {
            index,
            step_id: self.id.clone(),
            operation: self.operation_id.clone(),
        }
    }
}

/// Validates the recipe and resolves every step against the registry.
///
/// # Errors
///
/// Returns [`ExecutionError`] when the recipe is structurally invalid, names
/// an unknown operation, requires an ungranted capability, or carries
/// arguments the operation does not accept.
pub(super) fn resolve<'registry>(
    recipe: &Recipe,
    registry: &'registry OperationRegistry,
    capabilities: &CapabilitySet,
) -> Result<Vec<PreparedStep<'registry>>, ExecutionError> {
    recipe
        .validate()
        .map_err(|error| ExecutionError::global(ExecutionFailure::InvalidRecipe(error)))?;
    resolve_steps(recipe, registry, capabilities)
}

/// Applies every check that depends on this particular call.
///
/// # Errors
///
/// Returns [`ExecutionError`] when the recipe or input exceeds the budget, the
/// input representation does not flow through the steps, or the run was
/// cancelled before the first invocation.
pub(super) fn check_runtime(
    prepared: &[PreparedStep<'_>],
    input: &Value,
    budget: ExecutionBudget,
    cancellation: &dyn Cancellation,
) -> Result<(), ExecutionError> {
    limits::check_initial(
        prepared.len(),
        crate::ValueSummary::from_value(input).size_bytes,
        budget,
    )
    .map_err(ExecutionError::global)?;
    check_type_flow(prepared, input.kind())?;
    if cancellation.is_cancelled() {
        return Err(ExecutionError::global(ExecutionFailure::Operation(
            OperationError::Cancelled,
        )));
    }
    Ok(())
}

fn resolve_steps<'registry>(
    recipe: &Recipe,
    registry: &'registry OperationRegistry,
    capabilities: &CapabilitySet,
) -> Result<Vec<PreparedStep<'registry>>, ExecutionError> {
    let mut prepared = Vec::with_capacity(recipe.steps.len());
    for (index, step) in recipe.steps.iter().enumerate() {
        if step.disabled {
            prepared.push(skeleton(step, None, Arguments::new()));
            continue;
        }
        let location = super::step_location(index, step);
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
        prepared.push(skeleton(step, Some(operation), arguments));
    }
    Ok(prepared)
}

fn skeleton<'registry>(
    step: &RecipeStep,
    operation: Option<&'registry dyn Operation>,
    arguments: Arguments,
) -> PreparedStep<'registry> {
    PreparedStep {
        id: step.id.clone(),
        operation_id: step.operation.clone(),
        disabled: step.disabled,
        breakpoint: step.breakpoint,
        operation,
        arguments,
    }
}

fn check_type_flow(
    prepared: &[PreparedStep<'_>],
    input_kind: ValueKind,
) -> Result<(), ExecutionError> {
    let mut previous_output = ValueConstraint::Exact(input_kind);
    let mut index = 0;
    while index < prepared.len() {
        if prepared[index].disabled {
            index += 1;
            continue;
        }
        let operation = prepared[index]
            .operation
            .expect("enabled steps are resolved");
        let location = prepared[index].location(index);

        if flow::is_fork(&prepared[index].operation_id) {
            if !output_satisfies_input(&previous_output, &operation.spec().input) {
                return Err(mismatch(location));
            }
            let merge_index = find_merge_for_prepared(index, prepared).unwrap_or(prepared.len());
            validate_fork_body(prepared, index + 1, merge_index)?;
            previous_output = ValueConstraint::Exact(ValueKind::Text);
            index = if merge_index < prepared.len() {
                merge_index + 1
            } else {
                merge_index
            };
            continue;
        }

        if flow::is_merge(&prepared[index].operation_id) {
            index += 1;
            continue;
        }

        if !output_satisfies_input(&previous_output, &operation.spec().input) {
            return Err(mismatch(location));
        }
        previous_output = operation.spec().output.clone();
        index += 1;
    }
    Ok(())
}

fn validate_fork_body(
    prepared: &[PreparedStep<'_>],
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
        if step.disabled {
            continue;
        }
        let body_op = step.operation.expect("enabled body steps are resolved");
        if !output_satisfies_input(&body_prev, &body_op.spec().input) {
            return Err(mismatch(step.location(body_index)));
        }
        body_prev = body_op.spec().output.clone();
    }
    Ok(())
}

fn mismatch(location: StepLocation) -> ExecutionError {
    ExecutionError::at_step(
        ExecutionFailure::InputKindMismatch,
        location,
        crate::ExecutionTrace::default(),
    )
}

fn find_merge_for_prepared(fork_index: usize, prepared: &[PreparedStep<'_>]) -> Option<usize> {
    let ids: Vec<_> = prepared
        .iter()
        .map(|step| step.operation_id.clone())
        .collect();
    let merge_all: Vec<bool> = prepared
        .iter()
        .map(|step| match step.arguments.get("merge_all") {
            Some(ArgumentValue::Boolean(value)) => *value,
            _ => true,
        })
        .collect();
    let disabled: Vec<bool> = prepared.iter().map(|step| step.disabled).collect();
    flow::find_merge_index(fork_index, &ids, &merge_all, &disabled)
}

fn output_satisfies_input(output: &ValueConstraint, input: &ValueConstraint) -> bool {
    // A kind flows into a step when the step accepts it outright, or when the
    // model defines a conversion into something the step does accept. The
    // reference converts between dish types rather than refusing, and a check
    // that demanded an exact match would reject recipes that run there.
    ValueKind::ALL.iter().copied().all(|kind| {
        !output.accepts(kind)
            || ValueKind::ALL
                .iter()
                .copied()
                .any(|target| kind.converts_to(target) && input.accepts(target))
    })
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
