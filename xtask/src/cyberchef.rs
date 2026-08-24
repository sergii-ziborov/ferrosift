//! Reference-oracle tasks.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::run_streaming;

const VERSION: &str = "11.3.0";
const TAG: &str = "v11.3.0";
const COMMIT: &str = "d24ba1afce2e3a080308b5df7db033332fe94a1a";
const UPSTREAM: &str = "https://github.com/gchq/CyberChef.git";

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["setup"] => setup(),
        ["generate"] => generate(),
        ["verify"] => verify(),
        ["gap", rest @ ..] => gap(rest),
        other => {
            eprintln!("unknown cyberchef task: {}", other.join(" "));
            ExitCode::FAILURE
        }
    }
}

/// Repository root, one level above this crate.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn oracle_dir() -> PathBuf {
    repo_root().join("tools").join("cyberchef-oracle")
}

/// Where the pinned checkout lives, honouring the environment override.
fn checkout_dir() -> PathBuf {
    std::env::var_os("FERROSIFT_CYBERCHEF_DIR").map_or_else(
        || {
            oracle_dir()
                .join("vendor")
                .join(format!("cyberchef-v{VERSION}"))
        },
        PathBuf::from,
    )
}

/// Clones the reference at the pinned commit and installs its dependencies.
///
/// The checkout is never committed: it is large, it is upstream's, and pinning
/// it by commit makes re-fetching reproducible.
fn setup() -> ExitCode {
    let checkout = checkout_dir();
    if checkout.exists() {
        eprintln!("checkout already present at {}", checkout.display());
        return verify();
    }
    let Some(parent) = checkout.parent() else {
        eprintln!("cannot determine a parent directory for the checkout");
        return ExitCode::FAILURE;
    };
    if let Err(error) = std::fs::create_dir_all(parent) {
        eprintln!("cannot create {}: {error}", parent.display());
        return ExitCode::FAILURE;
    }
    let target = checkout.to_string_lossy().to_string();
    let steps: [(&str, Vec<&str>, Option<&str>); 3] = [
        (
            "git",
            vec!["clone", "--no-checkout", UPSTREAM, &target],
            None,
        ),
        ("git", vec!["-C", &target, "checkout", COMMIT], None),
        ("npm", vec!["ci"], Some(&target)),
    ];
    for (program, arguments, directory) in steps {
        if !run_streaming(program, &arguments, directory) {
            return ExitCode::FAILURE;
        }
    }
    verify()
}

/// Confirms the checkout sits exactly on the pinned tag and commit.
fn verify() -> ExitCode {
    let checkout = checkout_dir();
    if !checkout.exists() {
        eprintln!(
            "reference checkout missing at {}\nrun: cargo xtask cyberchef setup",
            checkout.display()
        );
        return ExitCode::FAILURE;
    }
    let target = checkout.to_string_lossy().to_string();
    eprintln!("expecting {TAG} {COMMIT}");
    if !run_streaming("git", &["-C", &target, "rev-parse", "HEAD"], None) {
        return ExitCode::FAILURE;
    }
    if run_streaming(
        "cargo",
        &["test", "-p", "ferrosift-operations", "--test", "corpus"],
        Some(&repo_root().to_string_lossy()),
    ) && run_streaming(
        "cargo",
        &[
            "test",
            "-p",
            "ferrosift-operations",
            "--test",
            "differential",
        ],
        Some(&repo_root().to_string_lossy()),
    ) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Regenerates both pinned fixtures from the reference.
fn generate() -> ExitCode {
    for script in ["generate-suite.mjs", "generate-corpus.mjs"] {
        let path = oracle_dir().join(script).to_string_lossy().to_string();
        if !run_streaming("node", &[&path], None) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Reports which reference operations are still unimplemented.
fn gap(extra: &[&str]) -> ExitCode {
    let path = oracle_dir().join("gap.mjs").to_string_lossy().to_string();
    let mut arguments = vec![path.as_str()];
    arguments.extend_from_slice(extra);
    if run_streaming("node", &arguments, None) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
