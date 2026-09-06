//! Shared hosted service used by CLI and MCP adapters.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use ferrosift_compat::cyberchef;
use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{
    CapabilitySet, CompatibilityProfile, OperationId, OperationSpec, Recipe, SchemaVersion,
    TextEncoding, TextValue, Value,
};

use crate::{
    allowlist::PathAllowlist,
    artifact::{ArtifactMeta, ArtifactStore, StoreConfig},
    error::{HostError, HostResult},
    report::{ExecutionReport, InspectReport, SearchHit},
};

/// Representation supplied when opening input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputKind {
    /// Uninterpreted bytes.
    Bytes,
    /// Strict UTF-8 text.
    Text,
}

/// Serialized recipe dialects accepted by the host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeFormat {
    /// Native `FerroSift` recipe JSON.
    FerroSift,
    /// `CyberChef` 11.3 compact recipe JSON.
    CyberChefV11_3,
    /// `CyberChef` 11.4 compact recipe JSON.
    CyberChefV11_4,
}

/// Host process configuration.
#[derive(Clone, Debug)]
pub struct HostConfig {
    /// Directories that may be read by `open`.
    pub allowed_roots: Vec<PathBuf>,
    /// Artifact store ceilings.
    pub store: StoreConfig,
    /// Maximum bytes accepted for one open.
    pub max_input_bytes: u64,
    /// Maximum search hits returned.
    pub max_search_results: usize,
    /// Execution budget applied to validate/run.
    pub budget: ExecutionBudget,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            allowed_roots: Vec::new(),
            store: StoreConfig::default(),
            max_input_bytes: 16 * 1024 * 1024,
            max_search_results: 8,
            budget: ExecutionBudget {
                max_steps: ferrosift_compat::cyberchef::MAX_RECIPE_STEPS,
                max_input_bytes: 16 * 1024 * 1024,
                max_output_bytes: 64 * 1024 * 1024,
                max_expansion_ratio: 64,
                max_branches: 1_048_576,
                max_flow_depth: 64,
                max_operation_invocations: 10_000_000,
                max_total_bytes_processed: 256 * 1024 * 1024,
                max_transient_bytes: 256 * 1024 * 1024,
                max_work_units: 1 << 26,
            },
        }
    }
}

/// Request to open bytes or an allowlisted file.
#[derive(Clone, Debug)]
pub enum OpenRequest<'a> {
    /// Inline bytes supplied by the caller.
    Bytes {
        /// Raw payload.
        bytes: Vec<u8>,
        /// Representation to assign.
        kind: InputKind,
    },
    /// Path that must resolve under an allowed root.
    Path {
        /// Filesystem path.
        path: &'a Path,
        /// Representation to assign.
        kind: InputKind,
    },
}

/// Inspect options.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InspectRequest {
    /// When true, include a bounded preview.
    pub include_preview: bool,
}

/// Recipe execution request.
#[derive(Clone, Debug)]
pub struct RunRequest<'a> {
    /// Recipe bytes.
    pub recipe: &'a [u8],
    /// Recipe dialect.
    pub format: RecipeFormat,
    /// Input artifact handle.
    pub input_artifact_id: &'a str,
}

/// Options for exporting a reproducible case.
#[derive(Clone, Debug)]
pub struct ExportReproRequest<'a> {
    /// Recipe bytes.
    pub recipe: &'a [u8],
    /// Recipe dialect.
    pub format: RecipeFormat,
    /// Raw input bytes.
    pub input: Vec<u8>,
    /// Input representation.
    pub input_kind: InputKind,
    /// Destination directory.
    pub directory: &'a Path,
    /// Provenance of the recorded expectation.
    pub expected_origin: crate::ExpectedOrigin,
    /// Optional pattern source stored beside the recipe.
    pub pattern: Option<String>,
    /// Whether secret-like argument names may be written.
    pub include_secrets: bool,
    /// Whether an existing directory may be replaced.
    pub overwrite: bool,
}

/// Shared hosted runtime for adapters.
pub struct HostService {
    registry: OperationRegistry,
    store: Mutex<ArtifactStore>,
    allowlist: PathAllowlist,
    max_input_bytes: u64,
    max_search_results: usize,
    budget: ExecutionBudget,
}

impl HostService {
    /// Builds a host over the portable operation catalog.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the registry or allowlist cannot be built.
    pub fn new(config: HostConfig) -> HostResult<Self> {
        let registry = ferrosift_operations::default_registry()
            .map_err(|error| HostError::new("host.registry.invalid", error.to_string()))?;
        let allowlist = PathAllowlist::new(config.allowed_roots)?;
        Ok(Self {
            registry,
            store: Mutex::new(ArtifactStore::new(config.store)),
            allowlist,
            max_input_bytes: config.max_input_bytes,
            max_search_results: config.max_search_results.max(1),
            budget: config.budget,
        })
    }

    /// Operation registry backing discovery and execution.
    #[must_use]
    pub const fn registry(&self) -> &OperationRegistry {
        &self.registry
    }

    /// Searches the catalog by id, name, description, or alias.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for spec in self.registry.catalog() {
            let mut matched_aliases = Vec::new();
            let id = spec.id.as_str().to_ascii_lowercase();
            let display = spec.display_name.to_ascii_lowercase();
            let category = spec.category.to_ascii_lowercase();
            let description = spec.description.to_ascii_lowercase();
            let mut matched = id.contains(&needle)
                || display.contains(&needle)
                || category.contains(&needle)
                || description.contains(&needle);
            for alias in &spec.aliases {
                if alias.name.to_ascii_lowercase().contains(&needle) {
                    matched = true;
                    matched_aliases.push(alias.name.clone());
                }
            }
            if matched {
                hits.push(SearchHit {
                    id: spec.id.as_str().to_owned(),
                    display_name: spec.display_name.clone(),
                    category: spec.category.clone(),
                    description: spec.description.clone(),
                    matched_aliases,
                });
            }
            if hits.len() >= self.max_search_results {
                break;
            }
        }
        hits
    }

    /// Returns one operation contract.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the operation id is unknown or malformed.
    pub fn describe(&self, operation: &str) -> HostResult<&OperationSpec> {
        let id = OperationId::new(operation)
            .map_err(|_| HostError::new("host.operation.unknown", operation.to_owned()))?;
        self.registry
            .get(&id)
            .map(ferrosift_core::Operation::spec)
            .ok_or_else(|| HostError::new("host.operation.unknown", operation.to_owned()))
    }

    /// Opens inline bytes or an allowlisted file into the artifact store.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] for access, size, UTF-8, or quota failures.
    pub fn open(&self, request: OpenRequest<'_>) -> HostResult<ArtifactMeta> {
        let (bytes, kind) = match request {
            OpenRequest::Bytes { bytes, kind } => {
                if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > self.max_input_bytes {
                    return Err(HostError::new(
                        "host.input.too_large",
                        format!("limit={}", self.max_input_bytes),
                    ));
                }
                (bytes, kind)
            }
            OpenRequest::Path { path, kind } => {
                (self.allowlist.read_limited(path, self.max_input_bytes)?, kind)
            }
        };
        let value = value_from_bytes(bytes, kind)?;
        self.store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(value)
    }

    /// Inspects a retained artifact.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the handle is unknown or expired.
    pub fn inspect(&self, artifact_id: &str, request: InspectRequest) -> HostResult<InspectReport> {
        let id = ArtifactStore::parse_id(artifact_id)?;
        let mut store = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let meta = store.meta(&id)?;
        let value = store.get_value(&id)?;
        let max_preview = store.config().max_preview_bytes;
        Ok(InspectReport::new(
            &meta,
            &value,
            max_preview,
            request.include_preview,
        ))
    }

    /// Validates a recipe without invoking operations.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the recipe is invalid for the chosen input kind.
    pub fn validate(
        &self,
        recipe_bytes: &[u8],
        format: RecipeFormat,
        input_kind: InputKind,
    ) -> HostResult<()> {
        let recipe = self.load_recipe(recipe_bytes, format)?;
        Executor::new(&self.registry)
            .validate(
                &recipe,
                &empty_value(input_kind),
                self.budget,
                &NeverCancelled,
                &CapabilitySet::new(),
            )
            .map_err(|error| map_execution(&error))?;
        Ok(())
    }

    /// Executes a recipe against a retained artifact and stores the result.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] for recipe, artifact, or execution failures.
    pub fn run(&self, request: &RunRequest<'_>) -> HostResult<ExecutionReport> {
        let recipe = self.load_recipe(request.recipe, request.format)?;
        let input_id = ArtifactStore::parse_id(request.input_artifact_id)?;
        let input = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_value(&input_id)?;
        let execution = Executor::new(&self.registry)
            .execute(
                &recipe,
                input,
                self.budget,
                &NeverCancelled,
                CapabilitySet::new(),
            )
            .map_err(|error| map_execution(&error))?;
        let meta = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(execution.value.clone())?;
        Ok(ExecutionReport::from_execution(&execution, &meta))
    }

    /// Runs a recipe on raw input and writes a reproducible case package.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] for recipe, execution, secret-policy, or write failures.
    pub fn export_repro(&self, request: &ExportReproRequest<'_>) -> HostResult<crate::ReproPackage> {
        let recipe = self.load_recipe(request.recipe, request.format)?;
        crate::repro::ensure_export_allowed(&recipe, request.include_secrets)?;
        let value = value_from_bytes(request.input.clone(), request.input_kind)?;
        let execution = Executor::new(&self.registry)
            .execute(
                &recipe,
                value,
                self.budget,
                &NeverCancelled,
                CapabilitySet::new(),
            )
            .map_err(|error| map_execution(&error))?;
        let package = crate::repro::build_package(&crate::repro::BuildPackageRequest {
            recipe_bytes: request.recipe,
            recipe_format: request.format,
            recipe: &recipe,
            input: &request.input,
            input_kind: request.input_kind,
            status: execution.status,
            expected: execution.value,
            budget: self.budget,
            expected_origin: request.expected_origin,
            pattern: request.pattern.clone(),
            include_secrets: request.include_secrets,
        })?;
        crate::repro::write_package(&package, request.directory, request.overwrite)?;
        Ok(package)
    }

    /// Replays a case package and compares against its expected result.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] when the package cannot be loaded or execution fails.
    pub fn check_repro(&self, directory: &Path) -> HostResult<crate::ReproCheckReport> {
        let package = crate::repro::read_package(directory)?;
        let format = crate::repro::parse_recipe_format(&package.manifest.recipe_format)?;
        let input_kind = crate::repro::parse_input_kind(&package.manifest.input_kind)?;
        let recipe = self.load_recipe(&package.recipe, format)?;
        let value = value_from_bytes(package.input.clone(), input_kind)?;
        let budget = package.manifest.budget.to_budget();
        let execution = Executor::new(&self.registry)
            .execute(
                &recipe,
                value,
                budget,
                &NeverCancelled,
                CapabilitySet::new(),
            )
            .map_err(|error| map_execution(&error))?;
        Ok(crate::repro::compare_observation(
            &package,
            execution.status,
            &execution.value,
        ))
    }

    fn load_recipe(&self, bytes: &[u8], format: RecipeFormat) -> HostResult<Recipe> {
        match format {
            RecipeFormat::FerroSift => {
                let recipe: Recipe = serde_json::from_slice(bytes)
                    .map_err(|error| HostError::new("host.recipe.malformed", error.to_string()))?;
                if recipe.schema_version != SchemaVersion::CURRENT.get() {
                    return Err(HostError::new(
                        "host.recipe.schema_unsupported",
                        format!("schema_version={}", recipe.schema_version),
                    ));
                }
                Ok(recipe)
            }
            RecipeFormat::CyberChefV11_3 => {
                self.load_cyberchef(bytes, CompatibilityProfile::CyberChefV11_3)
            }
            RecipeFormat::CyberChefV11_4 => {
                self.load_cyberchef(bytes, CompatibilityProfile::CyberChefV11_4)
            }
        }
    }

    fn load_cyberchef(
        &self,
        bytes: &[u8],
        profile: CompatibilityProfile,
    ) -> HostResult<Recipe> {
        let report = cyberchef::import_recipe(bytes, profile, &self.registry)
            .map_err(|error| HostError::new(error.code(), error.to_string()))?;
        if let Some(first) = report.findings.first() {
            return Err(HostError::new(
                first.code,
                format!("step={}", first.source_step),
            ));
        }
        report
            .recipe
            .ok_or_else(|| HostError::new("host.recipe.empty", "no recipe produced"))
    }
}

fn value_from_bytes(bytes: Vec<u8>, kind: InputKind) -> HostResult<Value> {
    match kind {
        InputKind::Bytes => Ok(Value::Bytes(bytes)),
        InputKind::Text => String::from_utf8(bytes)
            .map(|text| {
                Value::Text(TextValue {
                    text,
                    encoding: TextEncoding::Utf8,
                })
            })
            .map_err(|error| HostError::new("host.input.invalid_utf8", error.to_string())),
    }
}

fn empty_value(kind: InputKind) -> Value {
    match kind {
        InputKind::Bytes => Value::Bytes(Vec::new()),
        InputKind::Text => Value::Text(TextValue {
            text: String::new(),
            encoding: TextEncoding::Utf8,
        }),
    }
}

fn map_execution(error: &ferrosift_core::ExecutionError) -> HostError {
    HostError::new(error.code(), error.to_string())
}
