//! Reproducible case packages for CI and agent hand-off.

use std::{
    fs,
    path::Path,
};

use ferrosift_core::{ExecutionBudget, ExecutionStatus, ValueSummary};
use ferrosift_model::{ArgumentValue, Recipe, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    error::{HostError, HostResult},
    service::{InputKind, RecipeFormat},
};

/// How the expected result was established.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedOrigin {
    /// Snapshot of this runtime's own output. Useful for regression, not proof.
    ObservedOnly,
    /// A human reviewed and accepted the expectation.
    UserApproved,
    /// Checked against an independent implementation or specification.
    IndependentlyVerified,
    /// Produced by a pinned external reference runtime.
    ReferenceRuntime,
}

/// On-disk layout written under a case directory.
#[derive(Clone, Debug)]
pub struct ReproPackage {
    /// Raw input bytes.
    pub input: Vec<u8>,
    /// Recipe bytes exactly as supplied for replay.
    pub recipe: Vec<u8>,
    /// Optional pattern source.
    pub pattern: Option<String>,
    /// Expected final value.
    pub expected: Value,
    /// Machine-readable case metadata.
    pub manifest: ReproManifest,
}

/// Manifest written as `manifest.json`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReproManifest {
    /// Stable schema for this package.
    pub schema: String,
    /// Host/runtime crate version that wrote the package.
    pub runtime_version: String,
    /// Recipe dialect used for replay.
    pub recipe_format: String,
    /// Input representation.
    pub input_kind: String,
    /// Digest of `input.bin`.
    pub input_digest_sha256: String,
    /// Digest of `recipe.json` bytes.
    pub recipe_digest_sha256: String,
    /// Digest of the expected value encoding.
    pub expected_digest_sha256: String,
    /// Optional pattern source digest.
    pub pattern_digest_sha256: Option<String>,
    /// Operation ids in recipe order.
    pub operation_ids: Vec<String>,
    /// Effective execution ceilings captured at export time.
    pub budget: BudgetSnapshot,
    /// Provenance of the expected result.
    pub expected_origin: ExpectedOrigin,
    /// Whether the recipe may contain secret arguments.
    pub includes_secrets: bool,
    /// Execution status observed when the expectation was recorded.
    pub observed_status: String,
}

/// Serializable subset of [`ExecutionBudget`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_field_names)] // mirrors ExecutionBudget field names on purpose
pub struct BudgetSnapshot {
    /// Maximum recipe steps.
    pub max_steps: usize,
    /// Maximum input bytes.
    pub max_input_bytes: u64,
    /// Maximum output bytes.
    pub max_output_bytes: u64,
    /// Maximum expansion ratio.
    pub max_expansion_ratio: u32,
    /// Maximum branches.
    pub max_branches: usize,
    /// Maximum flow depth.
    pub max_flow_depth: usize,
    /// Maximum operation invocations.
    pub max_operation_invocations: u64,
    /// Maximum total bytes processed.
    pub max_total_bytes_processed: u64,
    /// Maximum transient bytes.
    pub max_transient_bytes: u64,
    /// Maximum work units.
    pub max_work_units: u64,
}

/// Result of replaying a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReproCheckReport {
    /// Whether input/recipe digests and expected value matched.
    pub passed: bool,
    /// Stable failure code when not passed.
    pub code: Option<&'static str>,
    /// Human detail.
    pub detail: String,
}

impl From<ExecutionBudget> for BudgetSnapshot {
    fn from(budget: ExecutionBudget) -> Self {
        Self {
            max_steps: budget.max_steps,
            max_input_bytes: budget.max_input_bytes,
            max_output_bytes: budget.max_output_bytes,
            max_expansion_ratio: budget.max_expansion_ratio,
            max_branches: budget.max_branches,
            max_flow_depth: budget.max_flow_depth,
            max_operation_invocations: budget.max_operation_invocations,
            max_total_bytes_processed: budget.max_total_bytes_processed,
            max_transient_bytes: budget.max_transient_bytes,
            max_work_units: budget.max_work_units,
        }
    }
}

impl BudgetSnapshot {
    /// Restores an [`ExecutionBudget`].
    #[must_use]
    pub const fn to_budget(&self) -> ExecutionBudget {
        ExecutionBudget {
            max_steps: self.max_steps,
            max_input_bytes: self.max_input_bytes,
            max_output_bytes: self.max_output_bytes,
            max_expansion_ratio: self.max_expansion_ratio,
            max_branches: self.max_branches,
            max_flow_depth: self.max_flow_depth,
            max_operation_invocations: self.max_operation_invocations,
            max_total_bytes_processed: self.max_total_bytes_processed,
            max_transient_bytes: self.max_transient_bytes,
            max_work_units: self.max_work_units,
        }
    }
}

/// Inputs required to assemble a repro package after execution.
pub struct BuildPackageRequest<'a> {
    /// Exact recipe bytes retained for replay.
    pub recipe_bytes: &'a [u8],
    /// Recipe dialect label.
    pub recipe_format: RecipeFormat,
    /// Parsed recipe used for operation id and secret scanning.
    pub recipe: &'a Recipe,
    /// Raw input bytes.
    pub input: &'a [u8],
    /// Input representation.
    pub input_kind: InputKind,
    /// Observed execution status.
    pub status: ExecutionStatus,
    /// Observed final value.
    pub expected: Value,
    /// Budget in force during the observation.
    pub budget: ExecutionBudget,
    /// Provenance of the expectation.
    pub expected_origin: ExpectedOrigin,
    /// Optional pattern source.
    pub pattern: Option<String>,
    /// Whether secret-like arguments may be retained.
    pub include_secrets: bool,
}

/// Refuses export when secret-like argument names are present without consent.
///
/// # Errors
///
/// Returns [`HostError`] when secrets would be written and are not allowed.
pub fn ensure_export_allowed(recipe: &Recipe, include_secrets: bool) -> HostResult<()> {
    let secret_names = secret_argument_names(recipe);
    if !secret_names.is_empty() && !include_secrets {
        return Err(HostError::new(
            "host.repro.secrets_blocked",
            format!(
                "refusing to export arguments named {}; pass include_secrets to override",
                secret_names.join(", ")
            ),
        ));
    }
    Ok(())
}

/// Builds a package from an observed execution.
///
/// # Errors
///
/// Returns [`HostError`] when the recipe contains secret-like arguments and
/// `include_secrets` is false.
pub fn build_package(request: &BuildPackageRequest<'_>) -> HostResult<ReproPackage> {
    ensure_export_allowed(request.recipe, request.include_secrets)?;
    let secret_names = secret_argument_names(request.recipe);
    let status_label = match request.status {
        ExecutionStatus::Completed => "completed".to_owned(),
        ExecutionStatus::Paused { step_index } => format!("paused:{step_index}"),
    };
    let pattern_digest = request
        .pattern
        .as_ref()
        .map(|source| digest_bytes(source.as_bytes()));
    let expected_digest = digest_value(&request.expected);
    Ok(ReproPackage {
        input: request.input.to_vec(),
        recipe: request.recipe_bytes.to_vec(),
        pattern: request.pattern.clone(),
        expected: request.expected.clone(),
        manifest: ReproManifest {
            schema: "ferrosift.repro.v1".to_owned(),
            runtime_version: env!("CARGO_PKG_VERSION").to_owned(),
            recipe_format: format_label(request.recipe_format).to_owned(),
            input_kind: input_kind_label(request.input_kind).to_owned(),
            input_digest_sha256: digest_bytes(request.input),
            recipe_digest_sha256: digest_bytes(request.recipe_bytes),
            expected_digest_sha256: expected_digest,
            pattern_digest_sha256: pattern_digest,
            operation_ids: request
                .recipe
                .steps
                .iter()
                .map(|step| step.operation.as_str().to_owned())
                .collect(),
            budget: BudgetSnapshot::from(request.budget),
            expected_origin: request.expected_origin,
            includes_secrets: !secret_names.is_empty(),
            observed_status: status_label,
        },
    })
}

/// Writes a package to `directory`. Refuses to overwrite unless requested.
///
/// # Errors
///
/// Returns [`HostError`] when the directory exists without overwrite, or I/O fails.
pub fn write_package(
    package: &ReproPackage,
    directory: &Path,
    overwrite: bool,
) -> HostResult<()> {
    if directory.exists() {
        if !overwrite {
            return Err(HostError::new(
                "host.repro.exists",
                format!("{}", directory.display()),
            ));
        }
        if directory.is_file() {
            return Err(HostError::new(
                "host.repro.not_directory",
                format!("{}", directory.display()),
            ));
        }
    } else {
        fs::create_dir_all(directory).map_err(|error| {
            HostError::new(
                "host.repro.write",
                format!("{}: {error}", directory.display()),
            )
        })?;
    }

    write_bytes(&directory.join("input.bin"), &package.input)?;
    write_bytes(&directory.join("recipe.json"), &package.recipe)?;
    if let Some(pattern) = &package.pattern {
        write_bytes(directory.join("pattern.hexpat").as_path(), pattern.as_bytes())?;
    }
    let expected = serde_json::to_vec_pretty(&package.expected)
        .map_err(|error| HostError::new("host.repro.serialize", error.to_string()))?;
    write_bytes(&directory.join("expected.json"), &expected)?;
    let manifest = serde_json::to_vec_pretty(&package.manifest)
        .map_err(|error| HostError::new("host.repro.serialize", error.to_string()))?;
    write_bytes(&directory.join("manifest.json"), &manifest)?;
    write_bytes(
        &directory.join("README.md"),
        repro_readme(package).as_bytes(),
    )?;
    Ok(())
}

/// Loads a package written by [`write_package`].
///
/// # Errors
///
/// Returns [`HostError`] when required files are missing or digests disagree.
pub fn read_package(directory: &Path) -> HostResult<ReproPackage> {
    let manifest: ReproManifest = read_json(&directory.join("manifest.json"))?;
    let input = fs::read(directory.join("input.bin")).map_err(|error| {
        HostError::new(
            "host.repro.read",
            format!("input.bin: {error}"),
        )
    })?;
    let recipe = fs::read(directory.join("recipe.json")).map_err(|error| {
        HostError::new(
            "host.repro.read",
            format!("recipe.json: {error}"),
        )
    })?;
    let expected: Value = read_json(&directory.join("expected.json"))?;
    let pattern = match fs::read_to_string(directory.join("pattern.hexpat")) {
        Ok(source) => Some(source),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(HostError::new(
                "host.repro.read",
                format!("pattern.hexpat: {error}"),
            ));
        }
    };

    if digest_bytes(&input) != manifest.input_digest_sha256 {
        return Err(HostError::new(
            "host.repro.input_digest_mismatch",
            "input.bin does not match manifest",
        ));
    }
    if digest_bytes(&recipe) != manifest.recipe_digest_sha256 {
        return Err(HostError::new(
            "host.repro.recipe_digest_mismatch",
            "recipe.json does not match manifest",
        ));
    }
    if digest_value(&expected) != manifest.expected_digest_sha256 {
        return Err(HostError::new(
            "host.repro.expected_digest_mismatch",
            "expected.json does not match manifest",
        ));
    }
    if let Some(source) = &pattern {
        let Some(expected_digest) = &manifest.pattern_digest_sha256 else {
            return Err(HostError::new(
                "host.repro.pattern_unexpected",
                "pattern.hexpat present but manifest has no pattern digest",
            ));
        };
        if &digest_bytes(source.as_bytes()) != expected_digest {
            return Err(HostError::new(
                "host.repro.pattern_digest_mismatch",
                "pattern.hexpat does not match manifest",
            ));
        }
    } else if manifest.pattern_digest_sha256.is_some() {
        return Err(HostError::new(
            "host.repro.pattern_missing",
            "manifest expects pattern.hexpat",
        ));
    }

    Ok(ReproPackage {
        input,
        recipe,
        pattern,
        expected,
        manifest,
    })
}

/// Compares an observed value and status against a package expectation.
#[must_use]
pub fn compare_observation(
    package: &ReproPackage,
    status: ExecutionStatus,
    value: &Value,
) -> ReproCheckReport {
    let status_label = match status {
        ExecutionStatus::Completed => "completed".to_owned(),
        ExecutionStatus::Paused { step_index } => format!("paused:{step_index}"),
    };
    if status_label != package.manifest.observed_status {
        return ReproCheckReport {
            passed: false,
            code: Some("host.repro.status_mismatch"),
            detail: format!(
                "expected status {} got {status_label}",
                package.manifest.observed_status
            ),
        };
    }
    if value != &package.expected {
        return ReproCheckReport {
            passed: false,
            code: Some("host.repro.expected_mismatch"),
            detail: format!(
                "expected kind={} size={} got kind={} size={}",
                package.expected.kind(),
                ValueSummary::from_value(&package.expected).size_bytes,
                value.kind(),
                ValueSummary::from_value(value).size_bytes
            ),
        };
    }
    ReproCheckReport {
        passed: true,
        code: None,
        detail: "ok".to_owned(),
    }
}

/// Parses recipe format labels stored in manifests.
///
/// # Errors
///
/// Returns [`HostError`] for unknown labels.
pub fn parse_recipe_format(label: &str) -> HostResult<RecipeFormat> {
    match label {
        "ferrosift" => Ok(RecipeFormat::FerroSift),
        "cyberchef-v11.3" => Ok(RecipeFormat::CyberChefV11_3),
        "cyberchef-v11.4" => Ok(RecipeFormat::CyberChefV11_4),
        other => Err(HostError::new(
            "host.repro.recipe_format_unknown",
            other.to_owned(),
        )),
    }
}

/// Parses input kind labels stored in manifests.
///
/// # Errors
///
/// Returns [`HostError`] for unknown labels.
pub fn parse_input_kind(label: &str) -> HostResult<InputKind> {
    match label {
        "bytes" => Ok(InputKind::Bytes),
        "text" => Ok(InputKind::Text),
        other => Err(HostError::new(
            "host.repro.input_kind_unknown",
            other.to_owned(),
        )),
    }
}

fn secret_argument_names(recipe: &Recipe) -> Vec<String> {
    let mut names = Vec::new();
    for step in &recipe.steps {
        for name in step.arguments.keys() {
            if looks_secret(name) {
                names.push(name.clone());
            }
        }
        for value in step.arguments.values() {
            collect_secret_keys(value, &mut names);
        }
    }
    names.sort();
    names.dedup();
    names
}

fn collect_secret_keys(value: &ArgumentValue, names: &mut Vec<String>) {
    match value {
        ArgumentValue::Map(map) => {
            for (key, nested) in map {
                if looks_secret(key) {
                    names.push(key.clone());
                }
                collect_secret_keys(nested, names);
            }
        }
        ArgumentValue::List(values) => {
            for nested in values {
                collect_secret_keys(nested, names);
            }
        }
        ArgumentValue::Boolean(_)
        | ArgumentValue::Integer(_)
        | ArgumentValue::Text(_)
        | ArgumentValue::Bytes(_) => {}
    }
}

fn looks_secret(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("passphrase")
        || lower == "key"
        || lower.ends_with("_key")
        || lower.starts_with("key_")
}

fn format_label(format: RecipeFormat) -> &'static str {
    match format {
        RecipeFormat::FerroSift => "ferrosift",
        RecipeFormat::CyberChefV11_3 => "cyberchef-v11.3",
        RecipeFormat::CyberChefV11_4 => "cyberchef-v11.4",
    }
}

fn input_kind_label(kind: InputKind) -> &'static str {
    match kind {
        InputKind::Bytes => "bytes",
        InputKind::Text => "text",
    }
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn digest_value(value: &Value) -> String {
    let encoded = serde_json::to_vec(value).unwrap_or_default();
    digest_bytes(&encoded)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> HostResult<()> {
    fs::write(path, bytes).map_err(|error| {
        HostError::new(
            "host.repro.write",
            format!("{}: {error}", path.display()),
        )
    })
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> HostResult<T> {
    let bytes = fs::read(path).map_err(|error| {
        HostError::new(
            "host.repro.read",
            format!("{}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        HostError::new(
            "host.repro.malformed",
            format!("{}: {error}", path.display()),
        )
    })
}

fn origin_label(origin: ExpectedOrigin) -> &'static str {
    match origin {
        ExpectedOrigin::ObservedOnly => "observed_only",
        ExpectedOrigin::UserApproved => "user_approved",
        ExpectedOrigin::IndependentlyVerified => "independently_verified",
        ExpectedOrigin::ReferenceRuntime => "reference_runtime",
    }
}

fn repro_readme(package: &ReproPackage) -> String {
    format!(
        "# FerroSift repro case\n\n\
         Replay with:\n\n\
         ```bash\n\
         ferrosift repro check --case .\n\
         ```\n\n\
         - recipe format: `{}`\n\
         - input kind: `{}`\n\
         - expected origin: `{}`\n\
         - observed status: `{}`\n\
         - includes secrets: {}\n",
        package.manifest.recipe_format,
        package.manifest.input_kind,
        origin_label(package.manifest.expected_origin),
        package.manifest.observed_status,
        package.manifest.includes_secrets,
    )
}
