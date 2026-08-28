//! Operations that move the executor's program counter.
//!
//! Three of them, and none touches the value. What they produce is a
//! [`FlowDirective`], and the executor is the only thing that acts on it — see
//! [`Operation::direct`].

use alloc::{string::String, vec};

use ferrosift_core::{FlowDirective, Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint};

#[cfg(feature = "text")]
use crate::args::boolean_argument;
use crate::args::{integer_argument, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

/// Reads a jump allowance the way the reference compares against one.
///
/// `maxJumps` is an ordinary JavaScript number there and the test is
/// `numJumps >= maxJumps`, so a negative allowance never permits a jump and an
/// enormous one is bounded by something else. Saturating at both ends says the
/// same thing without an error for an argument the reference accepts.
fn allowance(value: i128) -> u32 {
    u32::try_from(value).unwrap_or(if value < 0 { 0 } else { u32::MAX })
}

/// Sends the recipe to a named `Label`.
///
/// The one operation that makes the executor's counter go backwards, which is
/// why it carries a limit: a `Label` above a `Jump` is a loop, and a loop needs
/// a bound. The limit is shared with every other jump in the recipe, as the
/// reference shares it, and is cleared whenever a jump does not fire.
///
/// Jumping to a label that is not there is not an error. The reference looks it
/// up, finds nothing, and carries on — a recipe assembled by dragging steps
/// around can lose its destination, and refusing to run at all would be a
/// harsher answer than the one the recipe was written against.
pub struct Jump {
    spec: OperationSpec,
}

impl Jump {
    /// Creates the Jump flow-control operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.jump@1",
                display_name: "Jump",
                category: "Flow control",
                description: "Continues the recipe at the named Label, up to a maximum number of jumps.",
                cyberchef_alias: Some("Jump"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: vec![
                    text_argument("label", "Name of the Label to continue at.", ""),
                    integer_argument(
                        "max_jumps",
                        "How many jumps the recipe may take before this one stops firing.",
                        10,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Jump {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Jump {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        Ok(input)
    }

    fn direct(
        &self,
        _value: &Value,
        arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<FlowDirective, OperationError> {
        context.ensure_active()?;
        Ok(FlowDirective::Jump {
            label: String::from(text_value(arguments, "label")?),
            max_jumps: allowance(crate::args::integer_value(arguments, "max_jumps")?),
        })
    }
}

/// Sends the recipe to a named `Label` when the value matches a pattern.
///
/// The condition is evaluated here rather than in the executor, which is the
/// point of [`Operation::direct`] existing: the regular expression engine is a
/// feature of this crate and `ferrosift-core` never sees a pattern.
///
/// An empty pattern tests nothing and is not a jump that failed — the reference
/// leaves its counter alone in that case, so the allowance is neither spent nor
/// refunded.
#[cfg(feature = "text")]
pub struct ConditionalJump {
    spec: OperationSpec,
}

#[cfg(feature = "text")]
impl ConditionalJump {
    /// Creates the Conditional Jump flow-control operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.conditional_jump@1",
                display_name: "Conditional Jump",
                category: "Flow control",
                description: "Continues the recipe at the named Label when the value matches a regular expression.",
                cyberchef_alias: Some("Conditional Jump"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: vec![
                    text_argument(
                        "pattern",
                        "Regular expression to test the value against.",
                        "",
                    ),
                    boolean_argument("invert", "Jump when the pattern does not match.", false),
                    text_argument("label", "Name of the Label to continue at.", ""),
                    integer_argument(
                        "max_jumps",
                        "How many jumps the recipe may take before this one stops firing.",
                        10,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

#[cfg(feature = "text")]
impl Default for ConditionalJump {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "text")]
impl Operation for ConditionalJump {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        Ok(input)
    }

    fn direct(
        &self,
        value: &Value,
        arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<FlowDirective, OperationError> {
        context.ensure_active()?;
        let pattern = text_value(arguments, "pattern")?;
        if pattern.is_empty() {
            // Nothing was tested, so nothing is decided -- including the
            // allowance, which the reference reaches only through a branch this
            // case never enters.
            return Ok(FlowDirective::Next);
        }
        let subject = super::section::as_text(value);
        let matched = super::section::matches(pattern, &subject)?;
        if matched == crate::args::boolean_value(arguments, "invert")? {
            return Ok(FlowDirective::NotTaken);
        }
        Ok(FlowDirective::Jump {
            label: String::from(text_value(arguments, "label")?),
            max_jumps: allowance(crate::args::integer_value(arguments, "max_jumps")?),
        })
    }
}

/// Ends the recipe here, answering with the value in hand.
///
/// Inside a Fork branch or a Subsection tranche this ends that piece rather
/// than the run, because the reference gives each one its own recipe to return
/// from.
pub struct Return {
    spec: OperationSpec,
}

impl Return {
    /// Creates the Return flow-control operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.return@1",
                display_name: "Return",
                category: "Flow control",
                description: "Ends execution at this point and returns the current value.",
                cyberchef_alias: Some("Return"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Return {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Return {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        Ok(input)
    }

    fn direct(
        &self,
        _value: &Value,
        _arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<FlowDirective, OperationError> {
        context.ensure_active()?;
        Ok(FlowDirective::Stop)
    }
}

#[cfg(test)]
mod tests {
    use super::allowance;

    #[test]
    fn an_allowance_saturates_rather_than_failing() {
        assert_eq!(allowance(10), 10);
        assert_eq!(allowance(0), 0);
        assert_eq!(allowance(-1), 0);
        assert_eq!(allowance(i128::from(u64::MAX)), u32::MAX);
    }
}
