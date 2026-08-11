//! Deterministic catalog listing.

use std::io::Write;

use ferrosift_core::OperationRegistry;

use crate::error::CliError;

pub fn run(registry: &OperationRegistry, output: &mut dyn Write) -> Result<(), CliError> {
    for specification in registry.catalog() {
        writeln!(output, "{}", specification.id.as_str()).map_err(CliError::write)?;
    }
    Ok(())
}
