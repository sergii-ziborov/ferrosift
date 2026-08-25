//! JavaScript-semantics fixture tasks.
//!
//! The `CyberChef` oracle pins whole operations against a pinned reference.
//! This pins the language those operations are written in, against Node, which
//! is a narrower question and a cheaper one to keep honest.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::run_streaming;

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["generate"] => generate(),
        other => {
            eprintln!("unknown jscompat task: {}", other.join(" "));
            eprintln!("expected: generate");
            ExitCode::FAILURE
        }
    }
}

fn generate() -> ExitCode {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        .join("tools")
        .join("jscompat-oracle")
        .join("generate.mjs")
        .to_string_lossy()
        .to_string();
    if run_streaming("node", &[&script], None) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
