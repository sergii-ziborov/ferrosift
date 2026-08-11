//! Machine-readable operation description.

use std::io::Write;

use ferrosift_core::OperationRegistry;
use ferrosift_model::OperationId;

use crate::error::CliError;

pub fn run(
    registry: &OperationRegistry,
    operation: &str,
    output: &mut dyn Write,
) -> Result<(), CliError> {
    let id = OperationId::new(operation).map_err(|_| unknown(operation))?;
    let operation = registry.get(&id).ok_or_else(|| unknown(operation))?;
    serde_json::to_writer_pretty(&mut *output, operation.spec())
        .map_err(|error| CliError::new("cli.output.serialization", error.to_string()))?;
    writeln!(output).map_err(CliError::write)
}

fn unknown(operation: &str) -> CliError {
    CliError::new("cli.operation.unknown", operation)
}
