//! Conversion of one preserved source step into portable recipe IR.

use alloc::{format, string::String, vec::Vec};

use ferrosift_core::{Operation, OperationRegistry};
use ferrosift_model::{CompatibilityProfile, RecipeStep, StepId};
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::{
    arguments::{ArgumentIssue, import_arguments},
    error::ImportError,
    finding::CompatibilityFinding,
};

/// What to say when a name resolves in no profile, or not in this one.
///
/// A `&'static str` per profile rather than a formatted string: the finding
/// detail is part of a stable surface, and the set of profiles is small and
/// known at compile time.
const fn profile_detail(profile: CompatibilityProfile) -> &'static str {
    match profile {
        CompatibilityProfile::CyberChefV11_3 => "operation has no exact CyberChef 11.3 alias",
        CompatibilityProfile::CyberChefV11_4 => "operation has no exact CyberChef 11.4 alias",
        CompatibilityProfile::Native => "operation has no exact alias in the requested profile",
    }
}

pub(crate) fn map_step(
    index: usize,
    raw: &JsonValue,
    profile: CompatibilityProfile,
    registry: &OperationRegistry,
    findings: &mut Vec<CompatibilityFinding>,
) -> Result<Option<RecipeStep>, ImportError> {
    let JsonValue::Object(object) = raw else {
        findings.push(finding(
            "compat.cyberchef.step_not_object",
            index,
            None,
            "recipe step must be a JSON object",
        ));
        return Ok(None);
    };

    let finding_count = findings.len();
    let operation_name = read_operation_name(object, index, findings);
    let arguments = read_arguments(object, index, operation_name, findings);
    let disabled = read_flag(object, "disabled", index, operation_name, findings);
    let breakpoint = read_flag(object, "breakpoint", index, operation_name, findings);
    report_unknown_fields(object, index, operation_name, findings);
    if findings.len() != finding_count {
        return Ok(None);
    }
    let (Some(operation_name), Some(arguments), Some(disabled), Some(breakpoint)) =
        (operation_name, arguments, disabled, breakpoint)
    else {
        return Ok(None);
    };

    let Some(operation) = registry.resolve_alias(profile, operation_name) else {
        // The detail names the profile because that is what decides the
        // answer: an operation the reference introduced in 11.4 is genuinely
        // unknown to 11.3, and a message that did not say which was asked
        // would read as "FerroSift does not have this".
        findings.push(finding(
            "compat.cyberchef.unknown_operation",
            index,
            Some(operation_name),
            profile_detail(profile),
        ));
        return Ok(None);
    };

    let Some(arguments) = map_arguments(index, operation_name, arguments, operation, findings)
    else {
        return Ok(None);
    };
    let id = StepId::new(format!("cc-{index:04}")).map_err(|_| ImportError::InvalidRecipe)?;
    Ok(Some(RecipeStep {
        id,
        operation: operation.spec().id.clone(),
        arguments,
        disabled,
        breakpoint,
    }))
}

fn read_operation_name<'a>(
    object: &'a JsonMap<String, JsonValue>,
    index: usize,
    findings: &mut Vec<CompatibilityFinding>,
) -> Option<&'a str> {
    match object.get("op") {
        None => {
            findings.push(finding(
                "compat.cyberchef.missing_op",
                index,
                None,
                "missing op field",
            ));
            None
        }
        Some(JsonValue::String(name)) => Some(name),
        Some(_) => {
            findings.push(finding(
                "compat.cyberchef.invalid_op",
                index,
                None,
                "op must be a string",
            ));
            None
        }
    }
}

fn read_arguments<'a>(
    object: &'a JsonMap<String, JsonValue>,
    index: usize,
    operation: Option<&str>,
    findings: &mut Vec<CompatibilityFinding>,
) -> Option<&'a [JsonValue]> {
    match object.get("args") {
        None => {
            findings.push(finding(
                "compat.cyberchef.missing_args",
                index,
                operation,
                "missing args field",
            ));
            None
        }
        Some(JsonValue::Array(arguments)) => Some(arguments),
        Some(_) => {
            findings.push(finding(
                "compat.cyberchef.invalid_args",
                index,
                operation,
                "args must be an array",
            ));
            None
        }
    }
}

fn read_flag(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
    index: usize,
    operation: Option<&str>,
    findings: &mut Vec<CompatibilityFinding>,
) -> Option<bool> {
    match object.get(field) {
        None => Some(false),
        Some(JsonValue::Bool(value)) => Some(*value),
        Some(_) => {
            let code = if field == "disabled" {
                "compat.cyberchef.invalid_disabled"
            } else {
                "compat.cyberchef.invalid_breakpoint"
            };
            findings.push(finding(
                code,
                index,
                operation,
                format!("{field} must be a boolean"),
            ));
            None
        }
    }
}

fn report_unknown_fields(
    object: &JsonMap<String, JsonValue>,
    index: usize,
    operation: Option<&str>,
    findings: &mut Vec<CompatibilityFinding>,
) {
    for field in object
        .keys()
        .filter(|field| !matches!(field.as_str(), "op" | "args" | "disabled" | "breakpoint"))
    {
        findings.push(finding(
            "compat.cyberchef.unknown_field",
            index,
            operation,
            format!("unknown step field: {field}"),
        ));
    }
}

fn map_arguments(
    index: usize,
    operation_name: &str,
    values: &[JsonValue],
    operation: &dyn Operation,
    findings: &mut Vec<CompatibilityFinding>,
) -> Option<ferrosift_model::Arguments> {
    match import_arguments(values, &operation.spec().arguments) {
        Ok(arguments) => Some(arguments),
        Err(issues) => {
            for issue in issues {
                let (code, explanation) = match issue {
                    ArgumentIssue::Extra { position } => (
                        "compat.cyberchef.extra_argument",
                        format!("extra positional argument at index {position}"),
                    ),
                    ArgumentIssue::Missing { name } => (
                        "compat.cyberchef.missing_argument",
                        format!("missing required argument: {name}"),
                    ),
                    ArgumentIssue::Type { name, expected } => (
                        "compat.cyberchef.argument_type",
                        format!("argument {name} does not match {expected:?}"),
                    ),
                    ArgumentIssue::NumberRange { name } => (
                        "compat.cyberchef.argument_number_range",
                        format!("argument {name} exceeds the JavaScript safe-integer range"),
                    ),
                    ArgumentIssue::Depth { name } => (
                        "compat.cyberchef.argument_depth",
                        format!("argument {name} exceeds the executable nesting limit"),
                    ),
                };
                findings.push(finding(code, index, Some(operation_name), explanation));
            }
            None
        }
    }
}

fn finding(
    code: &'static str,
    source_step: usize,
    original_operation: Option<&str>,
    explanation: impl Into<String>,
) -> CompatibilityFinding {
    CompatibilityFinding::error(code, source_step, original_operation, explanation)
}
