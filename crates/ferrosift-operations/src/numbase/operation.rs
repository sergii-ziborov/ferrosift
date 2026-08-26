use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{integer_argument, integer_value};
use crate::failure::failed;
use crate::value::{take_decimal, take_text, text as text_output};

use super::codec;

/// The radix argument, which both operations carry with the same default.
fn radix_argument() -> ArgumentSpec {
    integer_argument("radix", "The base to use, from 2 to 36.", 36)
}

/// Reads the radix, refusing one the reference has no alphabet for.
fn radix_value(arguments: &Arguments, code: &'static str) -> Result<u32, OperationError> {
    let radix = integer_value(arguments, "radix")?;
    if !codec::RADIX_RANGE.contains(&radix) {
        return Err(failed(code));
    }
    u32::try_from(radix).map_err(|_| failed(code))
}

/// A number written in another base.
pub struct ToBase {
    spec: OperationSpec,
}

impl ToBase {
    /// Creates the To Base operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_spec(
                "encoding.radix.encode@1",
                "To Base",
                "Writes a number in a given base, from 2 to 36.",
                "To Base",
                ValueConstraint::Exact(ValueKind::Decimal),
                ValueConstraint::Exact(ValueKind::Text),
                Some("encoding.radix.decode@1"),
            ),
        }
    }
}

impl Default for ToBase {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase {
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
        let radix = radix_value(arguments, "encoding.radix.encode.bad_radix")?;
        let value = take_decimal(input)?;

        context.ensure_active()?;
        let written = codec::to_base(&value, radix)
            .ok_or_else(|| failed("encoding.radix.encode.bad_radix"))?;
        Ok(text_output(written))
    }
}

/// A number read from another base.
pub struct FromBase {
    spec: OperationSpec,
}

impl FromBase {
    /// Creates the From Base operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_spec(
                "encoding.radix.decode@1",
                "From Base",
                "Reads a number written in a given base, from 2 to 36.",
                "From Base",
                ValueConstraint::Exact(ValueKind::Text),
                ValueConstraint::Exact(ValueKind::Decimal),
                Some("encoding.radix.encode@1"),
            ),
        }
    }
}

impl Default for FromBase {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase {
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
        let radix = radix_value(arguments, "encoding.radix.decode.bad_radix")?;
        let input = take_text(input)?;

        context.ensure_active()?;
        // The reference throws for a digit its alphabet has no place for, so
        // the recipe stops rather than carrying a not-a-number forward.
        let value = codec::from_base(&input, radix)
            .ok_or_else(|| failed("encoding.radix.decode.bad_digit"))?;
        Ok(Value::Decimal(value))
    }
}

/// Builds the specification the two operations share the shape of.
fn build_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    input: ValueConstraint,
    output: ValueConstraint,
    inverse: Option<&'static str>,
) -> OperationSpec {
    crate::spec::build(crate::spec::SpecDefinition {
        id,
        display_name,
        category: "Encoding",
        description,
        cyberchef_alias: Some(alias),
        input,
        output,
        arguments: vec![radix_argument()],
        inverse,
        classifications: None,
    })
}
