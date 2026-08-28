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

/// The same run, reporting the failure code rather than discarding it.
fn refusal(operation: &str, input: &str) -> Result<String, String> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let arguments: Arguments = [(
        "delimiter".to_owned(),
        ArgumentValue::Text("Space".to_owned()),
    )]
    .into_iter()
    .collect();
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
        .map_err(|error| error.code().to_string())
        .map(|outcome| match outcome.value {
            Value::Decimal(value) => value.to_fixed(),
            Value::Text(text) => text.text,
            other => panic!("unexpected output {other:?}"),
        })
}

/// An answer too large to keep is refused before it is built.
///
/// Exact addition brings both operands to the finer of the two exponents, so
/// the gap between the exponents is the answer's width: `1e10000000 +
/// 1e-10000000` is twenty-three characters in and twenty million digits out.
/// The executor already refused that, and measured in a release build it spent
/// **five seconds** producing the digits it then threw away.
///
/// Which refuses is therefore the whole assertion. `core.operation.…` is the
/// fold declining at the step that would cross the ceiling;
/// `core.executor.…` is the old behaviour, where the answer existed first.
/// The two are indistinguishable in every other way -- same input, same
/// rejection, only the seconds differ -- so the code is what a test can hold.
#[test]
fn a_sum_wider_than_the_budget_is_refused_by_the_operation() {
    for operation in [
        "math.sum@1",
        "math.subtract@1",
        "math.mean@1",
        "math.median@1",
        "math.stddev@1",
    ] {
        assert_eq!(
            refusal(operation, "1e10000000 1e-10000000"),
            Err("core.operation.output_limit_exceeded".to_owned()),
            "{operation} must decline before building the answer"
        );
    }
}

/// And nothing else is refused with it.
///
/// The floor the fold reads is a floor on purpose: acting on it means
/// declining, so an estimate that ever came in high would decline an answer the
/// budget would have taken. These are the shapes where a careless bound would
/// do exactly that -- a wide gap whose answer still fits, two values whose
/// exponents match so no rescaling happens at all, and a sum that cancels.
#[test]
fn an_answer_that_fits_is_still_produced() {
    assert_eq!(aggregate("math.sum@1", "0.1 0.2"), Ok("0.3".to_owned()));
    assert_eq!(
        aggregate("math.sum@1", "1e20 1"),
        Ok("100000000000000000001".to_owned())
    );
    assert_eq!(
        aggregate("math.subtract@1", "1e20 1e20"),
        Ok("0".to_owned())
    );
    // A gap of forty places: wide enough that a bound working from the
    // exponents alone would have to reach for it, and small enough to keep.
    let wide = aggregate("math.sum@1", "1e20 1e-20").expect("a 41-digit answer fits");
    assert_eq!(wide.len(), 42, "{wide}");
}

/// The same again for division, which was left out of the floor above.
///
/// The reasoning was that division "already refuses an out-of-range scale
/// before it computes any digits", and it does — against ten million. A scale
/// of five million is *in* range and computed in full, so `1e5000000 / 3` was
/// thirty-four seconds of work producing a five-million-digit answer that the
/// executor then refused for being five million digits long.
///
/// Found by the `bignumber` fuzz target once it was given both operands and a
/// seed corpus of exponent extremes, which is what those two changes were for.
/// As above, the code is the assertion: `core.operation.…` is the fold
/// declining first, `core.executor.…` is the old behaviour where the answer
/// existed before anyone objected to it.
#[test]
fn a_quotient_wider_than_the_budget_is_refused_by_the_operation() {
    for operation in ["math.divide@1", "math.mean@1", "math.stddev@1"] {
        assert_eq!(
            refusal(operation, "1e5000000 3"),
            Err("core.operation.output_limit_exceeded".to_owned()),
            "{operation} must decline before building the quotient"
        );
    }
    // And the root, which sits behind the division in standard deviation and
    // amplifies again: the root of a value at 10^10000000 has five million
    // digits, and reaching it builds a radicand with ten million.
    assert_eq!(
        refusal("math.stddev@1", "1e5000000 0"),
        Err("core.operation.output_limit_exceeded".to_owned())
    );
}

/// And division still answers everything that fits.
///
/// The floor is a floor: `1e20 / 3` has twenty-one digits above the point and
/// a bound that claimed twenty-two would refuse it. The exact-division cases
/// matter for the same reason from the other side — an answer shorter than the
/// scale difference suggests must not be pre-emptively refused.
#[test]
fn a_quotient_that_fits_is_still_produced() {
    assert_eq!(aggregate("math.divide@1", "1 2"), Ok("0.5".to_owned()));
    assert_eq!(aggregate("math.divide@1", "100 4"), Ok("25".to_owned()));
    assert_eq!(
        aggregate("math.divide@1", "1e20 1"),
        Ok("100000000000000000000".to_owned())
    );
    // A divisor whose coefficient is larger than the dividend's, so the
    // quotient sits one place *below* the scale difference. A floor computed
    // as "difference plus one" would refuse this.
    let short = aggregate("math.divide@1", "1e2 3e1").expect("a short quotient fits");
    assert!(short.starts_with("3.33"), "{short}");
    assert_eq!(aggregate("math.mean@1", "1 2 3 4"), Ok("2.5".to_owned()));
    assert_eq!(
        aggregate("math.stddev@1", "1 2 3 4"),
        Ok("1.1180339887498948482".to_owned())
    );
}

/// Zero is added by not adding it.
///
/// `x + 0` is `x`, and computing it as one was the last of the three: bringing
/// `1e5000000` down to zero's exponent materialises five million digits in
/// order to add nothing to them. `sum_min_len` had always reported no cost for
/// a zero operand — true of the answer and false of the work — so this is the
/// work being made to match what was already claimed about it.
#[test]
fn adding_zero_costs_nothing_and_changes_nothing() {
    assert_eq!(
        aggregate("math.sum@1", "1e20 0"),
        Ok("100000000000000000000".to_owned())
    );
    assert_eq!(aggregate("math.sum@1", "0 0.25"), Ok("0.25".to_owned()));
    assert_eq!(
        aggregate("math.subtract@1", "0 0.25"),
        Ok("-0.25".to_owned())
    );
    assert_eq!(
        aggregate("math.subtract@1", "0.25 0"),
        Ok("0.25".to_owned())
    );
    assert_eq!(aggregate("math.sum@1", "0 0"), Ok("0".to_owned()));
    assert_eq!(aggregate("math.subtract@1", "0 0"), Ok("0".to_owned()));
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

/// A remainder does not build the power of ten it is about to divide away.
///
/// `MOD` used to bring both operands to a common exponent, so `1e10000000 MOD
/// 2` produced a ten-million-digit integer in order to answer `0`, and
/// `1e-10000000 MOD 2` scaled the *modulus* by ten million places to conclude
/// that the dividend was already the answer. Both are twelve characters of
/// input and one character of output, so no output budget could see either.
///
/// The answers are what a reader can check; that they arrive at all is the
/// rest of the test. Before the fix this file did not finish.
#[test]
fn a_remainder_does_not_materialise_the_exponent() {
    // The dividend's exponent above the modulus's: a modular exponentiation
    // rather than a decimal that has to exist.
    assert_eq!(modulo(2, "Space", "1e10000000"), Ok("0".to_owned()));
    assert_eq!(modulo(3, "Space", "1e9999999"), Ok("1".to_owned()));
    assert_eq!(modulo(7, "Space", "-1e10000000"), Ok("-4".to_owned()));

    // The modulus larger than the dividend: the dividend is its own remainder,
    // decided from the scales without aligning anything. `MOD` renders through
    // `toString` rather than in full, so the answer is also visibly a number
    // that was never expanded.
    assert_eq!(
        modulo(2, "Space", "1e-10000000"),
        Ok("1e-10000000".to_owned())
    );

    // And the ordinary cases still answer what they answered.
    assert_eq!(modulo(3, "Space", "15 4 7"), Ok("0 1 1".to_owned()));
    assert_eq!(modulo(3, "Space", "-7"), Ok("-1".to_owned()));
}

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
