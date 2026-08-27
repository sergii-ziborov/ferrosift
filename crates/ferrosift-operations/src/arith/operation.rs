use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, DecimalValue, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::failure::failed;
use crate::jscompat::bignumber;
use crate::jscompat::delim::char_rep;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

/// The delimiter argument every operation in this family carries.
///
/// The reference offers six, and `char_rep` already resolves all six -- so the
/// argument is the token itself rather than a separator, and an unknown token
/// is refused rather than silently treated as literal text.
fn delimiter_argument() -> ferrosift_model::ArgumentSpec {
    text_argument(
        "delimiter",
        "Separator between numbers: Line feed, Space, Comma, Semi-colon, Colon, or CRLF.",
        "Space",
    )
}

/// One aggregation over a delimited list of numbers.
///
/// Seven operations that differ only in which fold they apply and what they
/// are called. Written once because writing them seven times would mean seven
/// chances to get the shared half wrong in one of them -- the reading of the
/// list, the empty-list answer, and the value the dish receives are the same
/// in all seven, and the reference has them the same too.
pub struct Aggregate {
    spec: OperationSpec,
    fold: fn(&[DecimalValue], u64) -> Result<Option<DecimalValue>, OperationError>,
    unknown_delimiter: &'static str,
}

/// What distinguishes one aggregation from another.
struct AggregateDefinition {
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    unknown_delimiter: &'static str,
    fold: fn(&[DecimalValue], u64) -> Result<Option<DecimalValue>, OperationError>,
}

impl Aggregate {
    fn from(definition: &AggregateDefinition) -> Self {
        Self {
            spec: build(SpecDefinition {
                id: definition.id,
                display_name: definition.display_name,
                category: "Arithmetic",
                description: definition.description,
                cyberchef_alias: Some(definition.alias),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Decimal),
                arguments: vec![delimiter_argument()],
                inverse: None,
                classifications: None,
            }),
            fold: definition.fold,
            unknown_delimiter: definition.unknown_delimiter,
        }
    }

    /// Adds a list of numbers.
    #[must_use]
    pub fn sum() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.sum@1",
            display_name: "Sum",
            description: "Adds a list of numbers, ignoring any item that is not one.",
            alias: "Sum",
            unknown_delimiter: "math.sum.unknown_delimiter",
            fold: codec::total,
        })
    }

    /// Subtracts every number after the first from the first.
    #[must_use]
    pub fn subtract() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.subtract@1",
            display_name: "Subtract",
            description: "Subtracts a list of numbers, ignoring any item that is not one.",
            alias: "Subtract",
            unknown_delimiter: "math.subtract.unknown_delimiter",
            fold: codec::difference,
        })
    }

    /// Multiplies a list of numbers.
    #[must_use]
    pub fn multiply() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.multiply@1",
            display_name: "Multiply",
            description: "Multiplies a list of numbers, ignoring any item that is not one.",
            alias: "Multiply",
            unknown_delimiter: "math.multiply.unknown_delimiter",
            fold: codec::product,
        })
    }

    /// Divides the first number by every number after it.
    #[must_use]
    pub fn divide() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.divide@1",
            display_name: "Divide",
            description: "Divides a list of numbers, ignoring any item that is not one.",
            alias: "Divide",
            unknown_delimiter: "math.divide.unknown_delimiter",
            fold: codec::quotient,
        })
    }

    /// The arithmetic mean.
    #[must_use]
    pub fn mean() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.mean@1",
            display_name: "Mean",
            description: "Averages a list of numbers, ignoring any item that is not one.",
            alias: "Mean",
            unknown_delimiter: "math.mean.unknown_delimiter",
            fold: codec::average,
        })
    }

    /// The middle value, or the mean of the middle two.
    #[must_use]
    pub fn median() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.median@1",
            display_name: "Median",
            description: "Finds the middle of a list of numbers, ignoring any item that is not one.",
            alias: "Median",
            unknown_delimiter: "math.median.unknown_delimiter",
            fold: codec::middle,
        })
    }

    /// The population standard deviation.
    #[must_use]
    pub fn standard_deviation() -> Self {
        Self::from(&AggregateDefinition {
            id: "math.stddev@1",
            display_name: "Standard Deviation",
            description: "Measures the spread of a list of numbers, ignoring any item that is not one.",
            alias: "Standard Deviation",
            unknown_delimiter: "math.stddev.unknown_delimiter",
            fold: codec::standard_deviation,
        })
    }
}

impl Operation for Aggregate {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let delimiter = char_rep(text_value(arguments, "delimiter")?, self.unknown_delimiter)?;
        let input = take_text(input)?;

        // What the executor would accept back, worked out before anything is
        // built. Exact addition can turn two short numbers into tens of
        // millions of digits, and measuring the answer afterwards means paying
        // for it first -- so the fold is handed the ceiling and stops at the
        // step that would cross it.
        let ceiling = context
            .budget()
            .output_ceiling(u64::try_from(input.len()).unwrap_or(u64::MAX));
        let values = codec::read_list(&input, delimiter);
        context.ensure_active()?;
        // A list with nothing in it has no answer, and the reference says so
        // with not-a-number rather than with an error: its fold returns
        // nothing and the operation substitutes `NaN`.
        let answer = (self.fold)(&values, ceiling)?.unwrap_or_else(DecimalValue::not_a_number);
        Ok(Value::Decimal(answer))
    }
}

/// Each number in a list reduced modulo one value.
///
/// The only operation here that is not an aggregation: it answers a list, not
/// a number, and it therefore renders the numbers itself. The reference joins
/// them with `Array.prototype.join`, which calls `toString` -- so a remainder
/// of a hundred-millionth comes out as `1e-8` here where every other operation
/// in this family would write it out in full.
pub struct Mod {
    spec: OperationSpec,
}

impl Mod {
    /// Creates the modulo operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "math.mod@1",
                display_name: "MOD",
                category: "Arithmetic",
                description: "Reduces each number in a list modulo a given value.",
                cyberchef_alias: Some("MOD"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument("modulus", "The value to reduce by; cannot be zero.", 2),
                    delimiter_argument(),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Mod {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Mod {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let modulus = DecimalValue::from(integer_value(arguments, "modulus")?);
        if modulus.is_zero() {
            // The reference throws here rather than answering not-a-number,
            // which is a different observable outcome: the recipe stops.
            return Err(failed("math.mod.zero_modulus"));
        }
        let delimiter = char_rep(
            text_value(arguments, "delimiter")?,
            "math.mod.unknown_delimiter",
        )?;
        let input = take_text(input)?;

        let values = codec::read_list(&input, delimiter);
        context.ensure_active()?;

        // Joined with a space whatever the delimiter was, which is the
        // reference's own asymmetry: the delimiter says how to read the input
        // and has no say in how the answer is written.
        let mut output = String::new();
        for value in &values {
            if !output.is_empty() {
                output.push(' ');
            }
            output.push_str(&bignumber::modulo(value, &modulus).to_notation());
        }
        Ok(text_output(output))
    }
}
