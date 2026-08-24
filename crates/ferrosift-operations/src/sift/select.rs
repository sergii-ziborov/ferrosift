use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{bytes, take_bytes};

use super::codec::{self, Nth};

/// Shared argument list for the two nth-byte operations.
fn nth_arguments(verb: &'static str) -> alloc::vec::Vec<ferrosift_model::ArgumentSpec> {
    vec![
        integer_argument("every", verb, 4),
        integer_argument("start", "First byte position to consider.", 0),
        boolean_argument(
            "each_line",
            "Restart the offset after every line feed.",
            false,
        ),
    ]
}

/// Keeps every nth byte.
pub struct TakeNthBytes {
    spec: OperationSpec,
}

impl TakeNthBytes {
    /// Creates the take-nth-bytes operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Bytes,
                UniformSpec {
                    id: "data.nth_bytes.take@1",
                    display_name: "Take nth bytes",
                    category: "Data",
                    description: "Keeps every nth byte, starting at a given offset.",
                    cyberchef_alias: "Take nth bytes",
                    arguments: nth_arguments("Keep one byte out of every this many."),
                },
            ),
        }
    }
}

impl Default for TakeNthBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for TakeNthBytes {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        run_nth(Nth::Take, input, arguments, context)
    }
}

/// Discards every nth byte.
pub struct DropNthBytes {
    spec: OperationSpec,
}

impl DropNthBytes {
    /// Creates the drop-nth-bytes operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Bytes,
                UniformSpec {
                    id: "data.nth_bytes.drop@1",
                    display_name: "Drop nth bytes",
                    category: "Data",
                    description: "Discards every nth byte, starting at a given offset.",
                    cyberchef_alias: "Drop nth bytes",
                    arguments: nth_arguments("Drop one byte out of every this many."),
                },
            ),
        }
    }
}

impl Default for DropNthBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DropNthBytes {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        run_nth(Nth::Drop, input, arguments, context)
    }
}

fn run_nth(
    mode: Nth,
    input: Value,
    arguments: &Arguments,
    context: &mut OperationContext<'_>,
) -> Result<Value, OperationError> {
    context.ensure_active()?;
    let input = take_bytes(input)?;
    let output = codec::take_or_drop_nth(
        &input,
        mode,
        integer_value(arguments, "every")?,
        integer_value(arguments, "start")?,
        boolean_value(arguments, "each_line")?,
        context,
    )?;
    Ok(bytes(output))
}

/// Reverses the input by byte, character, or line.
pub struct Reverse {
    spec: OperationSpec,
}

impl Reverse {
    /// Creates the reverse operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Bytes,
                UniformSpec {
                    id: "data.reverse@1",
                    display_name: "Reverse",
                    category: "Data",
                    description: "Reverses the input by byte, character, or line.",
                    cyberchef_alias: "Reverse",
                    arguments: vec![text_argument(
                        "by",
                        "Reversal unit: Byte, Character, or Line.",
                        "Character",
                    )],
                },
            ),
        }
    }
}

impl Default for Reverse {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Reverse {
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
        let scope = text_value(arguments, "by")?;
        let input = take_bytes(input)?;
        Ok(bytes(codec::reverse(&input, scope, context)?))
    }
}
