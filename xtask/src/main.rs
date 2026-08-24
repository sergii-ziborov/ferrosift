//! Development tasks for the `FerroSift` workspace.
//!
//! This crate is deliberately outside the workspace and outside every library
//! dependency graph: nothing it does is reachable from the shipped crates. It
//! exists so the reference oracle is reproducible by anyone, not only by
//! whoever happens to have a checkout lying around.

use std::process::{Command, ExitCode};

mod bench;
mod cyberchef;
mod ledger;

const USAGE: &str = "\
FerroSift development tasks

Usage:
  cargo xtask cyberchef setup      Clone the pinned reference checkout
  cargo xtask cyberchef generate   Regenerate the pinned fixtures
  cargo xtask cyberchef verify     Check the pin, then replay the fixtures
  cargo xtask cyberchef gap        List reference operations not yet implemented
  cargo xtask ledger generate      Rewrite the derived compatibility ledger
  cargo xtask ledger check         Fail when the committed ledger is stale

The reference checkout defaults to tools/cyberchef-oracle/vendor and can be
pointed elsewhere with FERROSIFT_CYBERCHEF_DIR.
";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let parts: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match parts.as_slice() {
        ["bench", rest @ ..] => bench::run(rest),
        ["cyberchef", rest @ ..] => cyberchef::run(rest),
        ["ledger", rest @ ..] => ledger::run(rest),
        ["--help" | "-h"] | [] => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown task: {}\n", other.join(" "));
            print!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Runs a command, streaming its output, and reports whether it succeeded.
fn run_streaming(program: &str, arguments: &[&str], directory: Option<&str>) -> bool {
    let mut command = Command::new(program);
    command.args(arguments);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    eprintln!("$ {program} {}", arguments.join(" "));
    match command.status() {
        Ok(status) => status.success(),
        Err(error) => {
            eprintln!("failed to run {program}: {error}");
            false
        }
    }
}
