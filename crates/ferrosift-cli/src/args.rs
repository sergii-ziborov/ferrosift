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
    Operations,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RecipeFormat {
    /// `FerroSift`'s versioned portable recipe JSON.
    #[value(name = "ferrosift")]
    FerroSift,
    /// `CyberChef` 11.3 compact recipe JSON.
    #[value(name = "cyberchef-v11.3")]
    CyberChefV11_3,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum InputKind {
    /// Uninterpreted input bytes.
    Bytes,
    /// Strict UTF-8 text input.
    Text,
}
