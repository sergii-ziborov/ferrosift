//! The two exemptions from the expansion ratio must stay narrow, and true.
//!
//! `OutputBehavior::InputIndependent` waives the ratio for operations whose
//! output does not come from their input, and `OutputBehavior::Reducer` waives
//! it for operations whose output is a summary of fixed size. These tests hold
//! both lines from both sides: an exempt operation must run where the ratio
//! would have refused it, every other operation must still be refused, and a
//! reducer must actually reduce.

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, OutputBehavior, Recipe, RecipeMetadata,
    RecipeStep, StepId, TextEncoding, TextValue, Value, ValueKind,
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
        max_transient_bytes: 256 * 1024 * 1024,
        max_work_units: 1 << 26,
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
    //
    // Both entries earn it the same way: the output is decided by the
    // arguments and the value handed in is discarded, so a ratio measured
    // against that input would be measuring nothing.
    assert_eq!(
        generators,
        vec![
            "text.debruijn@1".to_owned(),
            "text.xkcd_random@1".to_owned()
        ],
        "every operation waiving the expansion ratio should be named here"
    );
}

/// Runs one operation over `input` with its declared defaults, or reports that
/// it refused.
fn run_defaults(spec: &ferrosift_model::OperationSpec, input: &[u8]) -> Result<usize, String> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    // The declared defaults, which is what a caller who names only the
    // operation gets — and the only argument set a test over the whole catalog
    // can supply without knowing each one.
    let arguments: Arguments = spec
        .arguments
        .iter()
        .filter_map(|argument| {
            argument
                .default
                .clone()
                .map(|value| (argument.name.clone(), value))
        })
        .collect();
    let recipe = Recipe::new(
        vec![RecipeStep {
            id: StepId::new("s").expect("step id"),
            operation: spec.id.clone(),
            arguments,
            disabled: false,
            breakpoint: false,
        }],
        RecipeMetadata::default(),
    )
    .expect("recipe");

    Executor::new(&registry)
        .execute(
            &recipe,
            Value::Bytes(input.to_vec()),
            ExecutionBudget::generous(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|error| error.code().to_owned())
        // Measured as the bytes a caller would see, which is what the
        // executor's own ceiling measures too — a number that renders as three
        // characters occupies three whether it is held as one or as text.
        .map(|outcome| match outcome.value {
            Value::Bytes(bytes) => bytes.len(),
            Value::Text(text) => text.text.len(),
            other => {
                let kind = other.kind();
                match other.reinterpret(ValueKind::Text) {
                    Some(Value::Text(text)) => text.text.len(),
                    _ => panic!("no textual size for {kind:?}"),
                }
            }
        })
}

/// A reducer must reduce.
///
/// The declaration waives a check, so it has to be earned rather than
/// asserted. Every operation claiming the class is run over inputs differing
/// by a factor of two hundred and fifty-six, and the larger input must not
/// produce more output than the smaller one — which is what "a summary of
/// fixed size" means, stated so a test can ask it.
///
/// An operation that refuses both inputs is not evidence either way and is
/// skipped: several of these want a particular shape — two samples separated
/// by a delimiter, a bcrypt string — that arbitrary bytes are not. What is not
/// skipped is one that *answers* and grows.
#[test]
fn every_reducer_actually_reduces() {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let small = [0x61_u8; 16];
    let large = [0x61_u8; 16 * 256];

    let mut checked = 0;
    let mut skipped = Vec::new();
    for spec in registry
        .catalog()
        .filter(|spec| matches!(spec.output_behavior, OutputBehavior::Reducer))
    {
        let (Ok(short), Ok(long)) = (run_defaults(spec, &small), run_defaults(spec, &large)) else {
            skipped.push(spec.id.as_str().to_owned());
            continue;
        };
        assert!(
            long <= short,
            "{} declares itself a reducer and grew from {short} to {long} bytes \
             when its input grew 256-fold",
            spec.id
        );
        checked += 1;
    }

    // A floor, so a change that made every reducer refuse arbitrary bytes
    // would fail here rather than passing with nothing checked.
    assert!(
        checked >= 20,
        "only {checked} reducers were actually exercised; skipped: {skipped:?}"
    );
}

/// A digest of an empty input is an ordinary thing to ask for.
///
/// It was not. The ratio divides the output by the input, so a *constant*
/// output against an empty input is an infinite ratio — and the generous
/// default budget, a ratio of sixty-four, refused `SHA-512` of nothing
/// outright with `core.executor.expansion_ratio_exceeded`. The reducer class
/// is what fixes it, and this is the case that pins the fix.
#[test]
fn a_wide_digest_of_an_empty_input_is_not_an_expansion() {
    let registry = ferrosift_operations::default_registry().expect("registry");
    for (operation, size) in [("hash.sha2@1", "512"), ("hash.sha3@1", "512")] {
        let spec = registry
            .get(&OperationId::new(operation).expect("operation id"))
            .expect("the catalog holds it")
            .spec();
        let mut arguments: Arguments = spec
            .arguments
            .iter()
            .filter_map(|argument| {
                argument
                    .default
                    .clone()
                    .map(|value| (argument.name.clone(), value))
            })
            .collect();
        arguments.insert("size".to_owned(), ArgumentValue::Text(size.to_owned()));

        let recipe = Recipe::new(
            vec![RecipeStep {
                id: StepId::new("s").expect("step id"),
                operation: spec.id.clone(),
                arguments,
                disabled: false,
                breakpoint: false,
            }],
            RecipeMetadata::default(),
        )
        .expect("recipe");

        let outcome = Executor::new(&registry)
            .execute(
                &recipe,
                Value::Bytes(Vec::new()),
                ExecutionBudget::generous(),
                &NeverCancelled,
                CapabilitySet::new(),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{operation} of an empty input was refused: {}",
                    error.code()
                )
            });

        let Value::Text(text) = outcome.value else {
            panic!("expected a hex digest");
        };
        // 512 bits, written as hex.
        assert_eq!(text.text.len(), 128, "{operation}");
    }
}
