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
/// against the reference catalog to report what is still unimplemented, and
/// `tools/ledger/safety.mjs` derives the safety matrix from the rest.
///
/// Everything a caller needs to decide *what to expose to input it did not
/// choose* is here rather than in prose: which host capabilities an operation
/// requires, what a reviewer has classified it as, whether it is
/// deterministic, and how its output relates to its input. All four are
/// declared beside the operation and none of them is inferable from its name.
///
/// The `evidence` block above them is what the whole catalog stands on: the
/// provenance, the licence, where the per-operation conformance result is
/// published, where the measurements are, and which targets this build actually
/// compiled and ran. It appears once because it is one fact — it used to be
/// copied into every specification, where nobody could read it and it said the
/// same thing two hundred and fifty-four times.
fn json(registry: &OperationRegistry, output: &mut dyn Write) -> Result<(), CliError> {
    let entries: Vec<_> = registry.catalog().map(entry).collect();
    let document = serde_json::json!({
        "evidence": registry.evidence(),
        "operations": entries,
    });
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
    // Serialized through `serde` rather than through `Debug`, so the names are
    // the model's own snake-case ones and a reader of the JSON sees the same
    // spelling as a reader of the enum. Targets were the one field still
    // rendered through `Debug`, which made them the one field a reader could
    // not match against the evidence manifest beside them.
    let targets: Vec<_> = specification.targets.iter().collect();
    let capabilities: Vec<_> = specification.capabilities.iter().collect();
    let classifications: Vec<_> = specification.classifications.iter().collect();
    serde_json::json!({
        "id": specification.id.as_str(),
        "display_name": specification.display_name,
        "category": specification.category,
        "aliases": aliases,
        "targets": targets,
        "deterministic": specification.deterministic,
        "capabilities": capabilities,
        "classifications": classifications,
        "output_behavior": specification.output_behavior,
        "streaming": specification.streaming,
    })
}
