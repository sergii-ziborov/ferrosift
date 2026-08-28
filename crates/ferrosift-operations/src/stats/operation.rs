use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, NumberValue, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build_reducer};

/// Widens a count to the type the arithmetic runs in.
///
/// The reference holds every one of these as a JavaScript number, which *is*
/// an `f64`, so the narrowing at 2^53 is the reference's own and reproducing
/// it is the point. A count that large would need an input of that many bytes.
#[expect(
    clippy::cast_precision_loss,
    reason = "the reference computes in f64, so matching it requires the same widening"
)]
fn widen(count: u64) -> f64 {
    count as f64
}

/// How far a byte distribution departs from a flat one.
pub struct ChiSquare {
    spec: OperationSpec,
}

impl ChiSquare {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "analysis.chi_square@1",
                display_name: "Chi Square",
                category: "Analysis",
                description: "Measures how far the byte distribution is from uniform.",
                cyberchef_alias: Some("Chi Square"),
                input: ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text])),
                output: ValueConstraint::Exact(ValueKind::Number),
                arguments: Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ChiSquare {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ChiSquare {
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
        let bytes = crate::value::take_bytes(input)?;

        let mut counts = [0u64; 256];
        for byte in &bytes {
            counts[usize::from(*byte)] += 1;
        }

        // The expected count per byte value. With no input this is zero and
        // every term is skipped, so the result is zero rather than a division
        // by zero -- the reference reaches the same place by testing the count
        // rather than the length.
        let expected = widen(bytes.len() as u64) / 256.0;
        let mut total = 0.0_f64;
        for count in counts {
            if count > 0 {
                let difference = widen(count) - expected;
                total += difference * difference / expected;
            }
        }

        Ok(Value::Number(NumberValue::new(total)))
    }
}

/// The chance that two letters drawn from the text are the same.
pub struct IndexOfCoincidence {
    spec: OperationSpec,
}

impl IndexOfCoincidence {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "analysis.index_of_coincidence@1",
                display_name: "Index of Coincidence",
                category: "Analysis",
                description: "The probability that two letters drawn from the text match.",
                cyberchef_alias: Some("Index of Coincidence"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Number),
                arguments: Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for IndexOfCoincidence {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for IndexOfCoincidence {
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
        let input = crate::value::take_text_value(input)?;

        // Only the twenty-six letters count, and case is folded away first.
        let mut frequencies = [0u64; 26];
        for letter in input.text.chars() {
            let lowered = letter.to_ascii_lowercase();
            if lowered.is_ascii_lowercase() {
                frequencies[usize::from(lowered as u8 - b'a')] += 1;
            }
        }

        let mut coincidence = 0.0_f64;
        let mut density: u64 = 0;
        for count in frequencies {
            coincidence += widen(count) * (widen(count) - 1.0);
            density += count;
        }

        // Fewer than two letters would divide by zero or by a negative, so the
        // reference clamps the denominator rather than the result. At the
        // clamp the numerator is zero, so the answer is zero either way.
        let density = if density < 2 { 2.0 } else { widen(density) };
        let result = coincidence / (density * (density - 1.0));

        Ok(Value::Number(NumberValue::new(result)))
    }
}
