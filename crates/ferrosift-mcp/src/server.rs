//! MCP tool surface over [`ferrosift_host::HostService`].

use std::{path::PathBuf, sync::Arc};

use ferrosift_host::{
    CandidateCheck, CandidateRecipe, CandidatesRequest, HostService, InputKind, InspectRequest,
    OpenRequest, RecipeFormat, RunRequest,
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

/// Stdio MCP server backed by one hosted service.
#[derive(Clone)]
pub struct FerroSiftMcp {
    host: Arc<HostService>,
    #[allow(dead_code)] // retained for `#[tool_handler]` / tool routing
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Free-text query over ids, names, descriptions, and aliases.
    query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DescribeParams {
    /// Canonical versioned operation id, for example `encoding.hex.encode@1`.
    operation: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct OpenParams {
    /// Inline UTF-8 text to store. Mutually exclusive with `path` and `bytes_hex`.
    #[serde(default)]
    text: Option<String>,
    /// Inline bytes as lowercase hex. Mutually exclusive with `path` and `text`.
    #[serde(default)]
    bytes_hex: Option<String>,
    /// Allowlisted filesystem path to read.
    #[serde(default)]
    path: Option<String>,
    /// `bytes` or `text`. Defaults to `bytes` for path/hex and `text` for text.
    #[serde(default)]
    input_kind: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct InspectParams {
    /// Opaque artifact handle from `ferrosift_open` or `ferrosift_run`.
    artifact_id: String,
    /// When true, include a bounded preview (default false).
    #[serde(default)]
    include_preview: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ValidateParams {
    /// Recipe JSON text.
    recipe_json: String,
    /// `ferrosift`, `cyberchef-v11.3`, or `cyberchef-v11.4`.
    format: String,
    /// `bytes` or `text` representation for the first step.
    input_kind: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RunParams {
    /// Recipe JSON text.
    recipe_json: String,
    /// `ferrosift`, `cyberchef-v11.3`, or `cyberchef-v11.4`.
    format: String,
    /// Opaque input artifact handle.
    input_artifact_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CandidatesParams {
    /// Opaque input artifact handle shared by every candidate.
    input_artifact_id: String,
    /// Explicit hypotheses, at most eight.
    candidates: Vec<CandidateParams>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CandidateParams {
    /// Caller-chosen label for this hypothesis.
    id: String,
    /// `ferrosift`, `cyberchef-v11.3`, or `cyberchef-v11.4`.
    format: String,
    /// Recipe JSON text.
    recipe_json: String,
    /// Optional deterministic observations on a successful output.
    #[serde(default)]
    checks: Vec<CheckParams>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CheckParams {
    /// Output representation must match (`bytes`, `text`, …).
    ValueKind { expected: String },
    /// Exact logical payload size.
    SizeEq { bytes: u64 },
    /// Inclusive lower bound on logical size.
    SizeMin { bytes: u64 },
    /// Inclusive upper bound on logical size.
    SizeMax { bytes: u64 },
    /// Leading bytes of bytes/UTF-8 text, as hex.
    PrefixHex { hex: String },
    /// Value must be UTF-8 text (or UTF-8 bytes).
    Utf8Text,
}

#[tool_router]
impl FerroSiftMcp {
    /// Creates an MCP adapter over an existing hosted service.
    pub fn new(host: Arc<HostService>) -> Self {
        Self {
            host,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "ferrosift_search",
        description = "Search FerroSift operations by id, name, description, or CyberChef alias. Returns a small ranked list, never the full catalog."
    )]
    fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let hits = self.host.search(&params.query);
        json_result(&hits)
    }

    #[tool(
        name = "ferrosift_describe",
        description = "Return the machine-readable contract for one canonical operation id, including arguments and catalog metadata."
    )]
    fn describe(
        &self,
        Parameters(params): Parameters<DescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let spec = self
            .host
            .describe(&params.operation)
            .map_err(|error| host_error(&error))?;
        json_result(spec)
    }

    #[tool(
        name = "ferrosift_open",
        description = "Open allowlisted file bytes or small inline input into an opaque artifact handle. Prefer handles over copying binary payloads through the model context."
    )]
    fn open(&self, Parameters(params): Parameters<OpenParams>) -> Result<CallToolResult, McpError> {
        let (mode, kind) = open_mode(&params)?;
        let request = match &mode {
            OpenMode::Bytes(bytes) => OpenRequest::Bytes {
                bytes: bytes.clone(),
                kind,
            },
            OpenMode::Path(path) => OpenRequest::Path { path, kind },
        };
        let meta = self
            .host
            .open(request)
            .map_err(|error| host_error(&error))?;
        json_result(&meta_json(&meta))
    }

    #[tool(
        name = "ferrosift_inspect",
        description = "Inspect a retained artifact. Preview is off by default; enable include_preview only when a short hex/text window is required."
    )]
    fn inspect(
        &self,
        Parameters(params): Parameters<InspectParams>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .host
            .inspect(
                &params.artifact_id,
                InspectRequest {
                    include_preview: params.include_preview,
                },
            )
            .map_err(|error| host_error(&error))?;
        json_result(&report)
    }

    #[tool(
        name = "ferrosift_validate",
        description = "Validate a recipe against the catalog and input kind without executing operations."
    )]
    fn validate(
        &self,
        Parameters(params): Parameters<ValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        let format = parse_format(&params.format)?;
        let kind = parse_kind(&params.input_kind)?;
        self.host
            .validate(params.recipe_json.as_bytes(), format, kind)
            .map_err(|error| host_error(&error))?;
        json_result(&serde_json::json!({ "status": "valid" }))
    }

    #[tool(
        name = "ferrosift_run",
        description = "Execute a recipe on an artifact handle and retain the output as a new artifact. Returns a ferrosift.execution.v1 report with status, typed value, and bounded trace."
    )]
    fn run(&self, Parameters(params): Parameters<RunParams>) -> Result<CallToolResult, McpError> {
        let format = parse_format(&params.format)?;
        let report = self
            .host
            .run(&RunRequest {
                recipe: params.recipe_json.as_bytes(),
                format,
                input_artifact_id: &params.input_artifact_id,
            })
            .map_err(|error| host_error(&error))?;
        json_result(&report)
    }

    #[tool(
        name = "ferrosift_candidates",
        description = "Evaluate up to eight explicit recipe hypotheses on one retained artifact. Returns a ferrosift.candidates.v1 observation table (status, size, checks, error, handle). Check counts are not calibrated probabilities."
    )]
    fn candidates(
        &self,
        Parameters(params): Parameters<CandidatesParams>,
    ) -> Result<CallToolResult, McpError> {
        let recipes: Vec<CandidateRecipe> = params
            .candidates
            .into_iter()
            .map(|candidate| CandidateRecipe {
                id: candidate.id,
                format: candidate.format,
                recipe_json: candidate.recipe_json,
                checks: candidate
                    .checks
                    .into_iter()
                    .map(map_check)
                    .collect(),
            })
            .collect();
        let report = self
            .host
            .evaluate_candidates(&CandidatesRequest {
                input_artifact_id: &params.input_artifact_id,
                candidates: &recipes,
            })
            .map_err(|error| host_error(&error))?;
        json_result(&report)
    }
}

#[tool_handler(
    name = "ferrosift",
    version = "0.1.0-alpha.1",
    instructions = "FerroSift turns unknown payloads into reproducible transforms. Prefer ferrosift_search then ferrosift_describe, open samples with ferrosift_open, keep bytes behind artifact handles, and use ferrosift_run or ferrosift_candidates. Do not treat a successful decode as proof the hypothesis is correct. Candidate check counts are observations, not probabilities."
)]
impl ServerHandler for FerroSiftMcp {}

enum OpenMode {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

fn open_mode(params: &OpenParams) -> Result<(OpenMode, InputKind), McpError> {
    let provided = usize::from(params.text.is_some())
        + usize::from(params.bytes_hex.is_some())
        + usize::from(params.path.is_some());
    if provided != 1 {
        return Err(invalid(
            "provide exactly one of text, bytes_hex, or path",
        ));
    }
    if let Some(text) = &params.text {
        let kind = parse_kind_default(params.input_kind.as_deref(), InputKind::Text)?;
        return Ok((OpenMode::Bytes(text.as_bytes().to_vec()), kind));
    }
    if let Some(hex) = &params.bytes_hex {
        let kind = parse_kind_default(params.input_kind.as_deref(), InputKind::Bytes)?;
        return Ok((OpenMode::Bytes(parse_hex(hex)?), kind));
    }
    let path = PathBuf::from(params.path.as_ref().expect("checked"));
    let kind = parse_kind_default(params.input_kind.as_deref(), InputKind::Bytes)?;
    Ok((OpenMode::Path(path), kind))
}

fn parse_kind_default(raw: Option<&str>, default: InputKind) -> Result<InputKind, McpError> {
    match raw {
        None => Ok(default),
        Some("bytes") => Ok(InputKind::Bytes),
        Some("text") => Ok(InputKind::Text),
        Some(other) => Err(invalid(format!("unknown input_kind: {other}"))),
    }
}

fn parse_kind(raw: &str) -> Result<InputKind, McpError> {
    parse_kind_default(Some(raw), InputKind::Bytes)
}

fn parse_format(raw: &str) -> Result<RecipeFormat, McpError> {
    match raw {
        "ferrosift" => Ok(RecipeFormat::FerroSift),
        "cyberchef-v11.3" => Ok(RecipeFormat::CyberChefV11_3),
        "cyberchef-v11.4" => Ok(RecipeFormat::CyberChefV11_4),
        other => Err(invalid(format!("unknown recipe format: {other}"))),
    }
}

fn map_check(check: CheckParams) -> CandidateCheck {
    match check {
        CheckParams::ValueKind { expected } => CandidateCheck::ValueKind { expected },
        CheckParams::SizeEq { bytes } => CandidateCheck::SizeEq { bytes },
        CheckParams::SizeMin { bytes } => CandidateCheck::SizeMin { bytes },
        CheckParams::SizeMax { bytes } => CandidateCheck::SizeMax { bytes },
        CheckParams::PrefixHex { hex } => CandidateCheck::PrefixHex { hex },
        CheckParams::Utf8Text => CandidateCheck::Utf8Text,
    }
}

fn parse_hex(text: &str) -> Result<Vec<u8>, McpError> {
    let cleaned: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err(invalid("bytes_hex must contain an even number of digits"));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&cleaned[index..index + 2], 16)
                .map_err(|_| invalid("bytes_hex contains a non-hex digit"))
        })
        .collect()
}

fn meta_json(meta: &ferrosift_host::ArtifactMeta) -> serde_json::Value {
    serde_json::json!({
        "artifact_id": meta.id.as_str(),
        "value_kind": meta.kind,
        "size_bytes": meta.size_bytes,
        "digest_sha256": meta.digest_sha256,
    })
}

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(content)]))
}

fn host_error(error: &ferrosift_host::HostError) -> McpError {
    McpError::invalid_params(format!("{}: {}", error.code(), error.detail()), None)
}

fn invalid(message: impl Into<String>) -> McpError {
    McpError::invalid_params(message.into(), None)
}
