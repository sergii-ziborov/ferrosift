//! Flow control where the reference has nothing to compare against.
//!
//! `tests/flow.rs` pins the behaviour itself, byte for byte, against the
//! reference's own interpreter. What is here is the rest: the ceilings that
//! stop a loop, the one place `FerroSift` deliberately answers differently, and
//! the cross-step type check that a marker must not break.
//!
//! Nothing here restates what the fixture already proves.

use ferrosift_core::{ExecutionBudget, ExecutionStatus, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, Value,
};

mod support;

fn step(id: &str, operation: &str, arguments: &[(&str, ArgumentValue)]) -> RecipeStep {
    RecipeStep {
        id: StepId::new(id).expect("valid step id"),
        operation: OperationId::new(operation).expect("valid operation id"),
        arguments: arguments
            .iter()
            .map(|(name, value)| ((*name).into(), value.clone()))
            .collect::<Arguments>(),
        disabled: false,
        breakpoint: false,
    }
}

fn text(value: &str) -> ArgumentValue {
    ArgumentValue::Text(value.into())
}

fn run(
    steps: Vec<RecipeStep>,
    input: Value,
    budget: ExecutionBudget,
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    let registry = support::registry();
    let recipe = Recipe::new(steps, RecipeMetadata::default()).expect("valid recipe");
    Executor::new(&registry).execute(
        &recipe,
        input,
        budget,
        &NeverCancelled,
        CapabilitySet::new(),
    )
}

fn completed(steps: Vec<RecipeStep>, input: Value) -> String {
    let result = run(steps, input, support::budget()).expect("recipe should complete");
    assert_eq!(result.status, ExecutionStatus::Completed);
    match result.value {
        Value::Text(value) => value.text,
        Value::Bytes(bytes) => String::from_utf8(bytes).expect("output should read as text"),
        other => panic!("expected text output, got {:?}", other.kind()),
    }
}

/// A loop bounded by nothing but the recipe's own allowance still stops.
///
/// `max_jumps` is the reference's bound and is under the recipe's control, so a
/// recipe asking for a billion of them is asking for a billion invocations. The
/// invocation ceiling is what answers that, and it answers it as a budget
/// failure rather than by running out of time.
#[test]
fn a_runaway_loop_is_stopped_by_the_invocation_ceiling() {
    let error = run(
        vec![
            step("label", "flow.label@1", &[("name", text("top"))]),
            step("identity", "core.identity@1", &[]),
            step(
                "jump",
                "flow.jump@1",
                &[
                    ("label", text("top")),
                    ("max_jumps", ArgumentValue::Integer(1_000_000_000)),
                ],
            ),
        ],
        support::text("abc"),
        ExecutionBudget {
            max_operation_invocations: 64,
            ..support::budget()
        },
    )
    .expect_err("a loop past the invocation ceiling must be refused");
    assert_eq!(error.code(), "core.executor.invocation_limit_exceeded");
}

/// A jump whose allowance is spent gets it back once it stops firing.
///
/// One counter for the whole recipe, cleared whenever a jump does not fire —
/// which is why the second loop below runs its full allowance again rather than
/// inheriting the first one's exhaustion.
#[test]
fn a_spent_allowance_is_refunded_when_the_jump_stops_firing() {
    // Two loops of one jump each. Each drops a byte until its allowance runs
    // out, so a shared-and-never-cleared counter would leave three characters
    // and a cleared one leaves two.
    let output = completed(
        vec![
            step("first", "flow.label@1", &[("name", text("a"))]),
            step(
                "drop1",
                "data.drop_bytes@1",
                &[
                    ("start", ArgumentValue::Integer(0)),
                    ("length", ArgumentValue::Integer(1)),
                    ("apply_to_each_line", ArgumentValue::Boolean(false)),
                ],
            ),
            step(
                "jump1",
                "flow.jump@1",
                &[
                    ("label", text("a")),
                    ("max_jumps", ArgumentValue::Integer(1)),
                ],
            ),
            step("second", "flow.label@1", &[("name", text("b"))]),
            step(
                "drop2",
                "data.drop_bytes@1",
                &[
                    ("start", ArgumentValue::Integer(0)),
                    ("length", ArgumentValue::Integer(1)),
                    ("apply_to_each_line", ArgumentValue::Boolean(false)),
                ],
            ),
            step(
                "jump2",
                "flow.jump@1",
                &[
                    ("label", text("b")),
                    ("max_jumps", ArgumentValue::Integer(1)),
                ],
            ),
        ],
        support::text("abcdefg"),
    );
    assert_eq!(output, "efg");
}

/// A marker between two typed steps does not break the cross-step type check.
///
/// Regression. An operation declaring `Any` on both sides carried that forward
/// as "the next step might receive anything", and the check then demanded that
/// *every* kind flow — including the three with no byte form. A `Label`, a
/// `Comment` or an `Identity` in front of a step that wants text was refused
/// before the first invocation, which is a legal recipe rejected by a question
/// that could not be answered yes.
#[test]
fn a_marker_does_not_break_the_type_check() {
    for marker in ["core.identity@1", "flow.comment@1", "flow.label@1"] {
        let output = completed(
            vec![
                step("marker", marker, &[]),
                step("upper", "text.case.upper@1", &[("scope", text("All"))]),
            ],
            support::text("ab"),
        );
        assert_eq!(
            output, "AB",
            "{marker} must be transparent to the type check"
        );
    }
}

/// A `Subsection` selecting more spans than the branch ceiling is refused.
///
/// The same ceiling a `Fork` is held to, because it is the same question: how
/// many times may one step make the rest of the recipe run.
#[test]
fn a_subsection_is_bounded_by_the_branch_ceiling() {
    let error = run(
        vec![
            step(
                "section",
                "flow.subsection@1",
                &[
                    ("pattern", text("[a-z]")),
                    ("case_sensitive", ArgumentValue::Boolean(true)),
                    ("global", ArgumentValue::Boolean(true)),
                    ("ignore_errors", ArgumentValue::Boolean(false)),
                ],
            ),
            step("upper", "text.case.upper@1", &[("scope", text("All"))]),
        ],
        support::text("abcdefghij"),
        ExecutionBudget {
            max_branches: 4,
            ..support::budget()
        },
    )
    .expect_err("more sections than the ceiling must be refused");
    assert_eq!(error.code(), "core.executor.branch_limit_exceeded");
}

/// A pattern that is not a pattern is an error, as it is in the reference.
#[test]
fn a_malformed_section_pattern_is_refused() {
    let error = run(
        vec![
            step(
                "section",
                "flow.subsection@1",
                &[
                    ("pattern", text("(unclosed")),
                    ("case_sensitive", ArgumentValue::Boolean(true)),
                    ("global", ArgumentValue::Boolean(true)),
                    ("ignore_errors", ArgumentValue::Boolean(false)),
                ],
            ),
            step("upper", "text.case.upper@1", &[("scope", text("All"))]),
        ],
        support::text("abc"),
        support::budget(),
    )
    .expect_err("a malformed pattern must be refused");
    assert_eq!(error.code(), "flow.section.invalid_pattern");
}

/// A failing section aborts the run unless the recipe says otherwise.
#[test]
fn a_failing_section_aborts_by_default() {
    let error = run(
        vec![
            step(
                "section",
                "flow.subsection@1",
                &[
                    ("pattern", text("[a-z]+")),
                    ("case_sensitive", ArgumentValue::Boolean(true)),
                    ("global", ArgumentValue::Boolean(true)),
                    ("ignore_errors", ArgumentValue::Boolean(false)),
                ],
            ),
            step(
                "hex",
                "encoding.hex.decode@1",
                &[("delimiter", text("Auto"))],
            ),
        ],
        support::text("zz-11"),
        support::budget(),
    )
    .expect_err("a failing section must abort");
    assert!(
        error.code().starts_with("encoding.hex"),
        "expected the section's own failure, got {}",
        error.code()
    );
}

/// A failing section contributes nothing when the recipe asks for that.
///
/// A documented divergence, and the one place this file differs from
/// `flow.json` on purpose: the reference splices the failing tranche's *error
/// message* into the output, which is a debugging aid there and would be an
/// injection of unrelated English into the answer here. A failing Fork branch
/// has always contributed an empty string in `FerroSift`, and a failing section
/// now does the same thing for the same reason. See
/// `docs/compatibility/cyberchef-v11.3.0.md`.
#[test]
fn a_failing_section_contributes_nothing_when_errors_are_ignored() {
    let output = completed(
        vec![
            step(
                "section",
                "flow.subsection@1",
                &[
                    ("pattern", text("[a-z]+")),
                    ("case_sensitive", ArgumentValue::Boolean(true)),
                    ("global", ArgumentValue::Boolean(true)),
                    ("ignore_errors", ArgumentValue::Boolean(true)),
                ],
            ),
            step(
                "hex",
                "encoding.hex.decode@1",
                &[("delimiter", text("Auto"))],
            ),
        ],
        support::text("zz-11"),
    );
    assert_eq!(output, "-11");
}

/// The catalog contains exactly one operation that opens a section region.
///
/// `FlowDirective::Sections` is honoured where the executor intercepts a region
/// and refused everywhere else. That refusal is unreachable from a recipe as
/// long as this holds, which is what makes it a contract check rather than a
/// code path a caller can trip.
#[test]
fn only_the_subsection_operation_opens_a_section_region() {
    let registry = support::registry();
    let openers: Vec<&str> = registry
        .catalog()
        .filter(|spec| spec.id.as_str() == ferrosift_core::SUBSECTION_ID)
        .map(|spec| spec.id.as_str())
        .collect();
    assert_eq!(openers, [ferrosift_core::SUBSECTION_ID]);
}
