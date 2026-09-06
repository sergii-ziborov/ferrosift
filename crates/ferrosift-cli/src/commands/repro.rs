//! Export and replay reproducible case packages.

use std::{
    io::{Read, Write},
    path::Path,
};

use ferrosift_host::{
    ExpectedOrigin, ExportReproRequest, HostConfig, HostService, InputKind as HostInputKind,
    RecipeFormat as HostRecipeFormat,
};

use crate::{
    args::{ExpectedOriginArg, InputKind, RecipeFormat},
    error::CliError,
    io, limits,
};

/// Arguments for `repro export`.
pub struct ExportRequest<'a> {
    /// Recipe dialect.
    pub format: RecipeFormat,
    /// Input representation.
    pub input_kind: InputKind,
    /// Recipe path, or `-`.
    pub recipe_path: &'a Path,
    /// Input path, or `-`.
    pub input_path: &'a Path,
    /// Destination case directory.
    pub out_dir: &'a Path,
    /// Optional pattern path.
    pub pattern_path: Option<&'a Path>,
    /// Expected-result provenance.
    pub origin: ExpectedOriginArg,
    /// Allow secret-like argument names in the package.
    pub include_secrets: bool,
    /// Replace an existing case directory.
    pub overwrite: bool,
}

pub fn export(
    request: &ExportRequest<'_>,
    standard_input: &mut dyn Read,
    standard_output: &mut dyn Write,
) -> Result<(), CliError> {
    if request.recipe_path == Path::new("-") && request.input_path == Path::new("-") {
        return Err(CliError::new(
            "cli.io.stdin_conflict",
            "recipe and input cannot both use standard input",
        ));
    }
    if request.pattern_path == Some(Path::new("-"))
        && (request.recipe_path == Path::new("-") || request.input_path == Path::new("-"))
    {
        return Err(CliError::new(
            "cli.io.stdin_conflict",
            "pattern cannot share standard input with recipe or input",
        ));
    }

    let recipe = io::read_limited(
        request.recipe_path,
        standard_input,
        limits::RECIPE_BYTES,
        "cli.recipe.too_large",
    )?;
    let input = io::read_limited(
        request.input_path,
        standard_input,
        limits::INPUT_BYTES,
        "cli.input.too_large",
    )?;
    let pattern = request
        .pattern_path
        .map(|path| {
            let bytes = io::read_limited(
                path,
                standard_input,
                limits::RECIPE_BYTES,
                "cli.pattern.too_large",
            )?;
            String::from_utf8(bytes)
                .map_err(|error| CliError::new("cli.pattern.invalid_utf8", error.to_string()))
        })
        .transpose()?;

    let host = HostService::new(HostConfig::default())
        .map_err(|error| CliError::new(error.code(), error.detail()))?;
    let package = host
        .export_repro(&ExportReproRequest {
            recipe: &recipe,
            format: host_format(request.format),
            input,
            input_kind: host_kind(request.input_kind),
            directory: request.out_dir,
            expected_origin: host_origin(request.origin),
            pattern,
            include_secrets: request.include_secrets,
            overwrite: request.overwrite,
        })
        .map_err(|error| CliError::new(error.code(), error.detail()))?;

    io::write_line(
        standard_output,
        &format!(
            "wrote {} ({})",
            request.out_dir.display(),
            package.manifest.schema
        ),
    )
}

pub fn check(case_dir: &Path, standard_output: &mut dyn Write) -> Result<(), CliError> {
    let host = HostService::new(HostConfig::default())
        .map_err(|error| CliError::new(error.code(), error.detail()))?;
    let report = host
        .check_repro(case_dir)
        .map_err(|error| CliError::new(error.code(), error.detail()))?;
    if report.passed {
        io::write_line(standard_output, "passed")
    } else {
        Err(CliError::new(
            report.code.unwrap_or("host.repro.check_failed"),
            report.detail,
        ))
    }
}

const fn host_format(format: RecipeFormat) -> HostRecipeFormat {
    match format {
        RecipeFormat::FerroSift => HostRecipeFormat::FerroSift,
        RecipeFormat::CyberChefV11_3 => HostRecipeFormat::CyberChefV11_3,
        RecipeFormat::CyberChefV11_4 => HostRecipeFormat::CyberChefV11_4,
    }
}

const fn host_kind(kind: InputKind) -> HostInputKind {
    match kind {
        InputKind::Bytes => HostInputKind::Bytes,
        InputKind::Text => HostInputKind::Text,
    }
}

const fn host_origin(origin: ExpectedOriginArg) -> ExpectedOrigin {
    match origin {
        ExpectedOriginArg::ObservedOnly => ExpectedOrigin::ObservedOnly,
        ExpectedOriginArg::UserApproved => ExpectedOrigin::UserApproved,
        ExpectedOriginArg::IndependentlyVerified => ExpectedOrigin::IndependentlyVerified,
        ExpectedOriginArg::ReferenceRuntime => ExpectedOrigin::ReferenceRuntime,
    }
}
