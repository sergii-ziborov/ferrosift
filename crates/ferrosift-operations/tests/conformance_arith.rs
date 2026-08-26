//! What the list-arithmetic operations refuse, and what they answer instead.
//!
//! The corpus pins outputs, so it cannot pin a case where the reference throws
//! rather than producing one. MOD throws on a zero modulus, and that refusal
//! is as much a part of the contract as its answers: a recipe stops there.
//!
//! It also holds the distinction the corpus cannot show on its own -- that an
//! empty list is *not* a refusal. The reference's fold has no seed, so it
//! produces nothing, and the operation turns that into not-a-number and
//! carries on. A port that treated the two the same would be wrong about one
//! of them, and both look reasonable in isolation.

#![cfg(feature = "arithmetic")]

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, TextEncoding, TextValue, Value,
};

/// Runs one operation over a space-delimited list, or reports that it refused.
fn run(operation: &str, arguments: Arguments, input: &str) -> Result<String, ()> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let recipe = Recipe::new(
        vec![RecipeStep {
            id: StepId::new("s").expect("step id"),
            operation: OperationId::new(operation).expect("operation id"),
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
            Value::Text(TextValue {
                text: input.to_owned(),
                encoding: TextEncoding::Utf8,
            }),
            ExecutionBudget::generous(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|_| ())
        .and_then(|outcome| match outcome.value {
            // Every aggregation answers a number, and the comparison is
            // against the rendering the dish would hand on.
            Value::Decimal(value) => Ok(value.to_fixed()),
            Value::Text(text) => Ok(text.text),
            _ => Err(()),
        })
}

fn aggregate(operation: &str, input: &str) -> Result<String, ()> {
    let arguments: Arguments = [(
        "delimiter".to_owned(),
        ArgumentValue::Text("Space".to_owned()),
    )]
    .into_iter()
    .collect();
    run(operation, arguments, input)
}

fn modulo(modulus: i128, delimiter: &str, input: &str) -> Result<String, ()> {
    let arguments: Arguments = [
        ("modulus".to_owned(), ArgumentValue::Integer(modulus)),
        (
            "delimiter".to_owned(),
            ArgumentValue::Text(delimiter.to_owned()),
        ),
    ]
    .into_iter()
    .collect();
    run("math.mod@1", arguments, input)
}

const AGGREGATIONS: [&str; 7] = [
    "math.sum@1",
    "math.subtract@1",
    "math.multiply@1",
    "math.divide@1",
    "math.mean@1",
    "math.median@1",
    "math.stddev@1",
];

#[test]
fn a_zero_modulus_is_refused_rather_than_answered() {
    // The reference throws here. Answering not-a-number instead would let a
    // recipe carry on past a step that could not be performed.
    assert!(
        modulo(0, "Space", "15 4 7").is_err(),
        "a zero modulus has no remainder to give"
    );
}

#[test]
fn an_empty_list_answers_not_a_number_rather_than_refusing() {
    // The other half of the same distinction: nothing to fold is not an
    // error, and the recipe continues with not-a-number.
    for operation in AGGREGATIONS {
        assert_eq!(
            aggregate(operation, ""),
            Ok("NaN".to_owned()),
            "{operation} on an empty input"
        );
        assert_eq!(
            aggregate(operation, "apples pears"),
            Ok("NaN".to_owned()),
            "{operation} on an input holding no numbers at all"
        );
    }

    // MOD answers an empty string rather than not-a-number, because it joins
    // its results and there are none to join.
    assert_eq!(modulo(3, "Space", ""), Ok(String::new()));
}

#[test]
fn an_unknown_delimiter_is_refused() {
    // The reference offers six and its interface cannot express a seventh. A
    // recipe that names one is malformed rather than an instruction to split
    // on the literal text.
    for operation in AGGREGATIONS {
        let arguments: Arguments = [(
            "delimiter".to_owned(),
            ArgumentValue::Text("Pipe".to_owned()),
        )]
        .into_iter()
        .collect();
        assert!(
            run(operation, arguments, "1 2 3").is_err(),
            "{operation} should refuse a delimiter the reference has no name for"
        );
    }
    assert!(modulo(3, "Pipe", "1 2 3").is_err());
}

#[test]
fn a_single_item_is_itself_rather_than_folded_against_a_seed() {
    // The reference reduces without a seed. A seed of zero would make
    // subtraction answer the negation, and a seed of one would make division
    // answer the reciprocal -- both plausible, both wrong.
    assert_eq!(aggregate("math.subtract@1", "42"), Ok("42".to_owned()));
    assert_eq!(aggregate("math.divide@1", "42"), Ok("42".to_owned()));
    assert_eq!(aggregate("math.sum@1", "42"), Ok("42".to_owned()));
    assert_eq!(aggregate("math.multiply@1", "42"), Ok("42".to_owned()));
}

#[test]
fn the_arithmetic_is_exact_where_a_float_would_not_be() {
    // The reason these operations carry a decimal rather than a double. Each
    // of these is a value a float cannot hold, and the answers below are the
    // ones the reference gives.
    assert_eq!(aggregate("math.sum@1", "0.1 0.2"), Ok("0.3".to_owned()));
    assert_eq!(
        aggregate("math.sum@1", "9007199254740993 1"),
        Ok("9007199254740994".to_owned()),
        "one past 2^53, where a float counts by twos"
    );
    assert_eq!(
        aggregate("math.multiply@1", "123456789012345678901234567890 2"),
        Ok("246913578024691357802469135780".to_owned())
    );

    // And where it is not exact, it stops where the reference stops.
    assert_eq!(
        aggregate("math.divide@1", "1 3"),
        Ok("0.33333333333333333333".to_owned()),
        "twenty places, and no more"
    );
}

#[test]
fn a_remainder_is_written_the_way_the_reference_writes_it() {
    // MOD joins its answers itself, which in the reference means `toString`
    // and therefore exponential notation below a ten-millionth. Every other
    // operation here hands back a number and the dish writes it with
    // `toFixed`, which never does. Both are pinned by the corpus; this states
    // the difference in one place so a reader meets it before a mismatch does.
    assert_eq!(modulo(3, "Space", "0.00000001"), Ok("1e-8".to_owned()));
    assert_eq!(modulo(3, "Space", "0.000001"), Ok("0.000001".to_owned()));
    assert_eq!(
        aggregate("math.sum@1", "0.00000001"),
        Ok("0.00000001".to_owned()),
        "the same value through the dish, written out in full"
    );
}
