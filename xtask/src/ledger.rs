//! Compatibility-ledger tasks.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::run_streaming;

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["generate"] => scripts(&[&["generate.mjs"], &["safety.mjs"]]),
        // Three checks rather than one. The first regenerates the ledger and
        // refuses a stale copy; the second holds the not-implemented page to
        // the same standard, which nothing did until two operations stayed
        // listed as missing for several revisions after they were built; the
        // third does the same for the safety matrix, which is the one document
        // where being out of date about the code would matter most.
        ["check"] => scripts(&[
            &["check.mjs"],
            &["not-implemented.mjs"],
            &["safety.mjs", "--check"],
        ]),
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

/// Runs each script, and reports failure if any of them failed.
///
/// Every one runs even after an earlier failure, so a single invocation shows
/// everything that has drifted rather than the first thing.
fn scripts(invocations: &[&[&str]]) -> ExitCode {
    let mut failed = false;
    for invocation in invocations {
        if script(invocation) == ExitCode::FAILURE {
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn script(invocation: &[&str]) -> ExitCode {
    let Some((name, rest)) = invocation.split_first() else {
        return ExitCode::FAILURE;
    };
    let path = repo_root()
        .join("tools")
        .join("ledger")
        .join(name)
        .to_string_lossy()
        .to_string();
    let mut arguments = vec![path.as_str()];
    arguments.extend_from_slice(rest);
    if run_streaming("node", &arguments, None) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
