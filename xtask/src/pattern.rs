//! Pattern-language oracle and ecosystem-survey tasks.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::{capture, run_streaming};

/// The pattern collection `ImHex` itself ships.
///
/// **GPL-2.0**, and this repository is Apache-2.0, so nothing from it is
/// vendored. `crates/ferrosift-pattern/tests/ecosystem.rs` records each file's
/// path, size, content digest and verdict; the checkout stays gitignored
/// beside the other references and exists only while the survey runs.
const PATTERNS_UPSTREAM: &str = "https://github.com/WerWolv/ImHex-Patterns.git";

/// Pinned, so the published number is a claim anyone can check rather than a
/// snapshot of whatever `master` happened to be. Must match `COMMIT` in
/// `crates/ferrosift-pattern/tests/ecosystem.rs`; the survey says so if it
/// does not.
const PATTERNS_COMMIT: &str = "4b25356eb7bec31ad33d6b196e8173c832b195f1";

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["setup"] => setup(),
        ["survey"] => survey(true),
        ["check"] => survey(false),
        other => {
            eprintln!("unknown pattern task: {}", other.join(" "));
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn checkout() -> PathBuf {
    repo_root()
        .join("tools")
        .join("pattern-oracle")
        .join("vendor")
        .join("ImHex-Patterns")
}

/// Clones the pattern collection at the pinned commit.
fn setup() -> ExitCode {
    let target = checkout();
    let path = target.to_string_lossy().to_string();
    if !target.exists() {
        let Some(parent) = target.parent() else {
            eprintln!("cannot determine a parent directory for the checkout");
            return ExitCode::FAILURE;
        };
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!("cannot create {}: {error}", parent.display());
            return ExitCode::FAILURE;
        }
        // Shallow: the survey wants one revision's worth of files and nothing
        // about how they got there.
        if !run_streaming(
            "git",
            &[
                "clone",
                "--no-checkout",
                "--depth",
                "1",
                PATTERNS_UPSTREAM,
                &path,
            ],
            None,
        ) {
            return ExitCode::FAILURE;
        }
        if !run_streaming("git", &["-C", &path, "checkout", PATTERNS_COMMIT], None) {
            return ExitCode::FAILURE;
        }
    }
    verify(&path)
}

/// Confirms the checkout sits exactly on the pinned commit.
fn verify(path: &str) -> ExitCode {
    let Some(head) = capture("git", &["-C", path, "rev-parse", "HEAD"]) else {
        eprintln!("cannot read the checkout's HEAD at {path}");
        return ExitCode::FAILURE;
    };
    if head.trim() != PATTERNS_COMMIT {
        eprintln!(
            "checkout at {path} is on {}, not the pinned commit {PATTERNS_COMMIT}",
            head.trim()
        );
        return ExitCode::FAILURE;
    }
    eprintln!("ImHex-Patterns is at {PATTERNS_COMMIT}");
    ExitCode::SUCCESS
}

/// Runs the ecosystem survey, optionally rewriting the recorded fixture.
///
/// `check` replays it and fails on any difference; `survey` records what the
/// parser answers now. The two are one command because they are one run — the
/// only difference is whether the answer is compared or written down.
fn survey(record: bool) -> ExitCode {
    let target = checkout();
    if !target.join("patterns").is_dir() {
        eprintln!(
            "no ImHex-Patterns checkout at {}\nrun: cargo xtask pattern setup",
            target.display()
        );
        return ExitCode::FAILURE;
    }
    if verify(&target.to_string_lossy()) == ExitCode::FAILURE {
        return ExitCode::FAILURE;
    }
    if record {
        // Read by the test rather than passed as an argument: `cargo test`
        // has no way to hand one through to the binary it builds.
        unsafe { std::env::set_var("FERROSIFT_RECORD_SURVEY", "1") };
    }
    let root = repo_root().to_string_lossy().to_string();
    if run_streaming(
        "cargo",
        &[
            "test",
            "-p",
            "ferrosift-pattern",
            "--test",
            "ecosystem",
            "--",
            "--nocapture",
        ],
        Some(&root),
    ) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
