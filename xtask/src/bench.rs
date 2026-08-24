//! Benchmark tasks.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::run_streaming;

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["run"] => measure(),
        ["report"] => report(),
        ["all"] => {
            if measure() == ExitCode::SUCCESS {
                report()
            } else {
                ExitCode::FAILURE
            }
        }
        other => {
            eprintln!("unknown bench task: {}", other.join(" "));
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// The benchmark binaries, run one at a time.
///
/// `cargo bench` with no target tries to run the library's own (empty) bench
/// harness first and rejects Criterion's arguments, so each is named.
const BENCHES: [&str; 3] = ["dispatch", "encoding", "digest"];

fn measure() -> ExitCode {
    let directory = repo_root().join("bench").to_string_lossy().to_string();
    for bench in BENCHES {
        let arguments = [
            "bench",
            "--bench",
            bench,
            "--",
            "--warm-up-time",
            "1",
            "--measurement-time",
            "3",
        ];
        if !run_streaming("cargo", &arguments, Some(&directory)) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn report() -> ExitCode {
    let script = repo_root()
        .join("tools")
        .join("bench")
        .join("report.mjs")
        .to_string_lossy()
        .to_string();
    if run_streaming("node", &[&script], None) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
