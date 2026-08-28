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
    },
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
