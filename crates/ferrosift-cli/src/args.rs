//! Declarative command-line syntax.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "ferrosift",
    about = "Deterministic local-first data transformation",
    disable_help_subcommand = true
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List canonical built-in operation identifiers.
    Operations {
        /// Listing format.
        #[arg(long, value_enum, default_value_t = CatalogFormat::Plain)]
        format: CatalogFormat,
    },
    /// Describe one canonical operation as JSON.
    Describe {
        /// Canonical versioned operation identifier.
        operation: String,
    },
    /// Validate a recipe without invoking its operations.
    Validate {
        /// Serialized recipe format.
        #[arg(long, value_enum)]
        format: RecipeFormat,
        /// Representation supplied to the first recipe step.
        #[arg(long, value_enum)]
        input_kind: InputKind,
        /// Recipe path, or '-' for standard input.
        #[arg(long)]
        recipe: PathBuf,
    },
    /// Execute a recipe under fixed resource ceilings.
    Run {
        /// Serialized recipe format.
        #[arg(long, value_enum)]
        format: RecipeFormat,
        /// Representation supplied to the first recipe step.
        #[arg(long, value_enum)]
        input_kind: InputKind,
        /// Recipe path, or '-' for standard input.
        #[arg(long)]
        recipe: PathBuf,
        /// Input path, or '-' for standard input.
        #[arg(long)]
        input: PathBuf,
        /// Output path, or '-' for standard output.
        #[arg(long, default_value = "-")]
        output: PathBuf,
        /// How the final value is written.
        #[arg(long, value_enum, default_value_t = ResultFormat::Raw)]
        result_format: ResultFormat,
    },
    /// Validate or evaluate a hex pattern.
    Pattern {
        #[command(subcommand)]
        command: PatternCommand,
    },
    /// Export or replay a reproducible case package.
    Repro {
        #[command(subcommand)]
        command: ReproCommand,
    },
}

/// Pattern-language subcommands.
#[derive(Debug, Subcommand)]
pub enum PatternCommand {
    /// Parse a pattern without reading subject bytes.
    Validate {
        /// Pattern path, or '-' for standard input.
        #[arg(long)]
        pattern: PathBuf,
    },
    /// Evaluate a pattern against subject bytes.
    Run {
        /// Pattern path, or '-' for standard input.
        #[arg(long)]
        pattern: PathBuf,
        /// Subject bytes path, or '-' for standard input.
        #[arg(long)]
        input: PathBuf,
        /// Output path, or '-' for standard output.
        #[arg(long, default_value = "-")]
        output: PathBuf,
    },
}

/// Reproducible case subcommands.
#[derive(Debug, Subcommand)]
pub enum ReproCommand {
    /// Run a recipe and write a case directory for CI replay.
    Export {
        /// Serialized recipe format.
        #[arg(long, value_enum)]
        format: RecipeFormat,
        /// Representation supplied to the first recipe step.
        #[arg(long, value_enum)]
        input_kind: InputKind,
        /// Recipe path, or '-' for standard input.
        #[arg(long)]
        recipe: PathBuf,
        /// Input path, or '-' for standard input.
        #[arg(long)]
        input: PathBuf,
        /// Destination case directory.
        #[arg(long)]
        out_dir: PathBuf,
        /// Optional pattern source to store beside the recipe.
        #[arg(long)]
        pattern: Option<PathBuf>,
        /// Provenance of the recorded expectation.
        #[arg(long, value_enum, default_value_t = ExpectedOriginArg::ObservedOnly)]
        origin: ExpectedOriginArg,
        /// Allow exporting recipes with secret-like argument names.
        #[arg(long, default_value_t = false)]
        include_secrets: bool,
        /// Replace an existing case directory.
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    /// Replay a case directory and compare against its expected result.
    Check {
        /// Case directory written by `repro export`.
        #[arg(long)]
        case: PathBuf,
    },
}

/// Provenance labels accepted by `repro export`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ExpectedOriginArg {
    /// Snapshot of this runtime's own output.
    #[default]
    ObservedOnly,
    /// Human-reviewed expectation.
    UserApproved,
    /// Checked against an independent implementation.
    IndependentlyVerified,
    /// Produced by a pinned reference runtime.
    ReferenceRuntime,
}

/// How `operations` renders the catalog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum CatalogFormat {
    /// One canonical identifier per line.
    #[default]
    Plain,
    /// One JSON object per operation, with compatibility aliases.
    Json,
}

/// How `run` writes a completed value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ResultFormat {
    /// Bytes or UTF-8 text only, for shell pipelines.
    #[default]
    Raw,
    /// Tagged JSON envelope with status, value, and bounded trace.
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RecipeFormat {
    /// `FerroSift`'s versioned portable recipe JSON.
    #[value(name = "ferrosift")]
    FerroSift,
    /// `CyberChef` 11.3 compact recipe JSON.
    #[value(name = "cyberchef-v11.3")]
    CyberChefV11_3,
    /// `CyberChef` 11.4 compact recipe JSON.
    ///
    /// The same shape as 11.3 — the reference's whole recipe model is
    /// unchanged between the two — so this selects which operation *names*
    /// resolve. A recipe using an operation 11.4 introduced loads here and not
    /// as 11.3, which is a fact about the reference rather than about this
    /// port.
    #[value(name = "cyberchef-v11.4")]
    CyberChefV11_4,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum InputKind {
    /// Uninterpreted input bytes.
    Bytes,
    /// Strict UTF-8 text input.
    Text,
}
