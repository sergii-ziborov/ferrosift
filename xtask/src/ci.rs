//! The cheap deterministic gates CI runs, runnable in one command locally.
//!
//! CI has grown to eleven jobs across three workflows, and the ones that cost
//! nothing are scattered among the ones that cost minutes. That is fine for a
//! machine and bad for a person: the way to find out whether a change passes
//! became "push it and wait", and a formatting break in `xtask` reached the
//! default branch precisely because `cargo fmt --all` at the root does not see
//! a crate outside the workspace.
//!
//! So this is the workspace's own smoke test. Everything here is deterministic,
//! needs no network, and finishes in the time it takes to read a diff. What is
//! deliberately absent is everything that does not fit that description —
//! cross-compilation, the reference checkout, coverage, the fuzz build — which
//! have their own tasks and their own reasons to be slower.
//!
//! Every gate runs even after an earlier one fails, so one invocation shows
//! everything that is wrong rather than the first thing.

use std::process::ExitCode;

use crate::run_streaming;

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["check"] | [] => check(),
        other => {
            eprintln!(
                "unknown ci task: {}\n\nUsage:\n  cargo xtask ci check   Run the cheap gates CI runs\n",
                other.join(" ")
            );
            ExitCode::FAILURE
        }
    }
}

/// One gate: what to call it, and the command that decides it.
struct Gate {
    name: &'static str,
    program: &'static str,
    arguments: &'static [&'static str],
}

/// The manifests outside the workspace.
///
/// `cargo fmt --all` formats the workspace and nothing else, so these three are
/// invisible to it. Naming them here is what turns "I forgot xtask exists" from
/// a red default branch into a local failure.
const OUTSIDE: &[&str] = &["xtask/Cargo.toml", "bench/Cargo.toml", "fuzz/Cargo.toml"];

fn check() -> ExitCode {
    let mut failed = Vec::new();

    let gates: &[Gate] = &[
        Gate {
            name: "format (workspace)",
            program: "cargo",
            arguments: &["fmt", "--all", "--", "--check"],
        },
        Gate {
            name: "clippy",
            program: "cargo",
            arguments: &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        },
        Gate {
            name: "tests",
            program: "cargo",
            arguments: &["test", "--workspace", "--all-features"],
        },
        Gate {
            name: "documentation",
            program: "cargo",
            arguments: &["doc", "--workspace", "--no-deps"],
        },
    ];

    for gate in gates {
        eprintln!("== {} ==", gate.name);
        if !run_streaming(gate.program, gate.arguments, None) {
            failed.push(gate.name);
        }
    }

    for manifest in OUTSIDE {
        eprintln!("== format ({manifest}) ==");
        if !run_streaming(
            "cargo",
            &["fmt", "--manifest-path", manifest, "--", "--check"],
            None,
        ) {
            failed.push(manifest);
        }
    }

    // The generated documents last, because they are the ones a reader is most
    // likely to have edited by hand without meaning to.
    for task in [
        ["ledger", "check"],
        ["encoding", "check"],
        ["pattern", "check"],
    ] {
        eprintln!("== {} {} ==", task[0], task[1]);
        let result = match task[0] {
            "ledger" => crate::ledger::run(&task[1..]),
            "encoding" => crate::encoding::run(&task[1..]),
            _ => crate::pattern::run(&task[1..]),
        };
        if result != ExitCode::SUCCESS {
            failed.push(task[0]);
        }
    }

    if failed.is_empty() {
        eprintln!("\nevery cheap gate passed");
        return ExitCode::SUCCESS;
    }
    eprintln!("\nfailed: {}", failed.join(", "));
    eprintln!(
        "the slower gates are separate on purpose: `cargo xtask coverage check`, \
         `cargo xtask cyberchef verify`, and the cross-compilation jobs in CI"
    );
    ExitCode::FAILURE
}
