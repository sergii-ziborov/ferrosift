//! Deterministic catalog listing.

use std::io::Write;

use ferrosift_core::OperationRegistry;
use ferrosift_model::OperationSpec;

use crate::args::CatalogFormat;
use crate::error::CliError;

pub fn run(
    registry: &OperationRegistry,
    format: CatalogFormat,
    output: &mut dyn Write,
) -> Result<(), CliError> {
    match format {
        CatalogFormat::Plain => plain(registry, output),
        CatalogFormat::Json => json(registry, output),
    }
}

fn plain(registry: &OperationRegistry, output: &mut dyn Write) -> Result<(), CliError> {
    for specification in registry.catalog() {
        writeln!(output, "{}", specification.id.as_str()).map_err(CliError::write)?;
    }
    Ok(())
}

/// Emits the catalog as machine-readable JSON.
///
/// This is the seam external tooling reads: the oracle compares these aliases
/// against the reference catalog to report what is still unimplemented.
fn json(registry: &OperationRegistry, output: &mut dyn Write) -> Result<(), CliError> {
    let entries: Vec<_> = registry.catalog().map(entry).collect();
    let document = serde_json::json!({ "operations": entries });
    let rendered = serde_json::to_string_pretty(&document)
        .map_err(|error| CliError::new("cli.catalog.unserializable", error.to_string()))?;
    writeln!(output, "{rendered}").map_err(CliError::write)
}

fn entry(specification: &OperationSpec) -> serde_json::Value {
    let aliases: Vec<_> = specification
        .aliases
        .iter()
        .map(|alias| {
            serde_json::json!({
                "profile": format!("{:?}", alias.profile),
                "name": alias.name,
            })
        })
        .collect();
    let targets: Vec<_> = specification
        .targets
        .iter()
        .map(|target| format!("{target:?}"))
        .collect();
    serde_json::json!({
        "id": specification.id.as_str(),
        "display_name": specification.display_name,
        "category": specification.category,
        "aliases": aliases,
        "targets": targets,
        "deterministic": specification.deterministic,
    })
}
