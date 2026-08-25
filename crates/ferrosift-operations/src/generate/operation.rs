use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{integer_argument, integer_value};
use crate::spec::{SpecDefinition, build_generator};
use crate::value::{take_text, text as text_output};

use super::codec;

/// Generates a De Bruijn sequence.
///
/// The first operation in the catalog to declare that its output does not
/// depend on its input.
pub struct GenerateDeBruijnSequence {
    spec: OperationSpec,
}

impl GenerateDeBruijnSequence {
    /// Creates the De Bruijn sequence operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            // `build_generator` rather than `build`: the sequence is decided by
            // the two arguments, and the value handed in is discarded.
            spec: build_generator(SpecDefinition {
                id: "text.debruijn@1",
                display_name: "Generate De Bruijn Sequence",
                category: "Text",
                description: "Generates a cyclic sequence containing every subsequence of a given length.",
                cyberchef_alias: Some("Generate De Bruijn Sequence"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument("alphabet_size_k", "Symbols in the alphabet, 2 to 9.", 2),
                    integer_argument("key_length_n", "Length of each subsequence.", 3),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for GenerateDeBruijnSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for GenerateDeBruijnSequence {
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
        // The reference ignores its input entirely. Taking it anyway keeps the
        // value model honest about what was consumed, and keeps the declared
        // input kind meaningful.
        let _ = take_text(input)?;
        let k = integer_value(arguments, "alphabet_size_k")?;
        let n = integer_value(arguments, "key_length_n")?;
        Ok(text_output(codec::de_bruijn(k, n, context)?))
    }
}
