use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_bytes, text as text_output};

use super::codec::{self, Rot13Options, Sample};

/// Bytes in, a report out.
fn report_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Ciphers",
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(ValueKind::Bytes),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// The two arguments both brute forces share, plus the crib.
fn sample_arguments() -> Vec<ArgumentSpec> {
    vec![
        integer_argument("sample_length", "Bytes of input to rotate.", 100),
        integer_argument("sample_offset", "Where the sample starts.", 0),
        boolean_argument("print_amount", "Prefix each line with its shift.", true),
        text_argument("crib", "Only show shifts containing this text.", ""),
    ]
}

/// Reads the sample window shared by both operations.
fn sample(arguments: &Arguments) -> Result<Sample, OperationError> {
    let length = integer_value(arguments, "sample_length")?;
    let offset = integer_value(arguments, "sample_offset")?;
    Ok(Sample {
        // A negative length or offset slices to nothing, which is what the
        // reference's `slice` does with them.
        offset: usize::try_from(offset).unwrap_or(0),
        length: usize::try_from(length).unwrap_or(0),
    })
}

/// Every ROT13 shift, filtered by a crib.
pub struct Rot13BruteForce {
    spec: OperationSpec,
}

impl Rot13BruteForce {
    /// Creates the ROT13 brute-force operation.
    #[must_use]
    pub fn new() -> Self {
        let mut arguments = vec![
            boolean_argument("rotate_lower_case_chars", "Rotate a to z.", true),
            boolean_argument("rotate_upper_case_chars", "Rotate A to Z.", true),
            boolean_argument("rotate_numbers", "Rotate 0 to 9.", false),
        ];
        arguments.extend(sample_arguments());
        Self {
            spec: report_spec(
                "cipher.rot13.brute@1",
                "ROT13 Brute Force",
                "Lists every ROT13 shift whose result contains the crib.",
                "ROT13 Brute Force",
                arguments,
            ),
        }
    }
}

impl Default for Rot13BruteForce {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Rot13BruteForce {
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
        let options = Rot13Options {
            lower: boolean_value(arguments, "rotate_lower_case_chars")?,
            upper: boolean_value(arguments, "rotate_upper_case_chars")?,
            numbers: boolean_value(arguments, "rotate_numbers")?,
        };
        let window = sample(arguments)?;
        let print_amount = boolean_value(arguments, "print_amount")?;
        let crib = text_value(arguments, "crib")?.to_lowercase();
        let input = take_bytes(input)?;
        Ok(text_output(codec::rot13_brute(
            &input,
            options,
            window,
            print_amount,
            &crib,
            context,
        )?))
    }
}

/// Every ROT47 shift, filtered by a crib.
pub struct Rot47BruteForce {
    spec: OperationSpec,
}

impl Rot47BruteForce {
    /// Creates the ROT47 brute-force operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: report_spec(
                "cipher.rot47.brute@1",
                "ROT47 Brute Force",
                "Lists every ROT47 shift whose result contains the crib.",
                "ROT47 Brute Force",
                sample_arguments(),
            ),
        }
    }
}

impl Default for Rot47BruteForce {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Rot47BruteForce {
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
        let window = sample(arguments)?;
        let print_amount = boolean_value(arguments, "print_amount")?;
        let crib = text_value(arguments, "crib")?.to_lowercase();
        let input = take_bytes(input)?;
        Ok(text_output(codec::rot47_brute(
            &input,
            window,
            print_amount,
            &crib,
            context,
        )?))
    }
}
