//! Compatibility-ledger tasks.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::run_streaming;

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["generate"] => script("generate.mjs"),
        ["check"] => script("check.mjs"),
        other => {
            eprintln!("unknown ledger task: {}", other.join(" "));
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn script(name: &str) -> ExitCode {
    let path = repo_root()
        .join("tools")
        .join("ledger")
        .join(name)
        .to_string_lossy()
        .to_string();
    if run_streaming("node", &[&path], None) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
