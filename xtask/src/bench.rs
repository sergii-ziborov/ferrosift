//! Benchmark tasks.
//!
//! Benchmarks run in batches, and a batch is only worth re-running when the
//! code it measures has changed. At a hundred operations a full sweep is a
//! coffee break; at five hundred it is an afternoon, and an afternoon that
//! mostly re-measures untouched code is an afternoon nobody spends — which is
//! how published numbers go stale.
//!
//! Each batch therefore records the commit it measured, and `check` reports
//! which batches predate a change to the sources they cover. Criterion keeps
//! its results per group, so re-running one batch leaves the others intact and
//! the report is assembled from whatever is on disk.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::run_streaming;

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["run", rest @ ..] => measure(rest),
        ["report"] => report(),
        ["check"] => check(),
        ["stale"] => rerun_stale(),
        ["all"] => {
            if measure(&[]) == ExitCode::SUCCESS {
                report()
            } else {
                ExitCode::FAILURE
            }
        }
        other => {
            eprintln!("unknown bench task: {}", other.join(" "));
            eprintln!("expected: run [batch...] | stale | report | check | all");
            ExitCode::FAILURE
        }
    }
}

/// One benchmark binary and the sources whose change invalidates it.
///
/// The watch list is deliberately hand-written rather than inferred. A
/// generated list would be complete and useless: every operation depends on
/// the model and the executor, so everything would invalidate everything. What
/// is wanted is the narrower question — did the code this batch actually
/// measures change — and only a person can answer that.
struct Batch {
    name: &'static str,
    watches: &'static [&'static str],
}

/// Paths that invalidate every batch, because every batch runs through them.
const SHARED: &[&str] = &[
    "crates/ferrosift-model/src",
    "crates/ferrosift-core/src",
    "bench/src",
    "bench/Cargo.toml",
];

const BATCHES: &[Batch] = &[
    Batch {
        name: "encoding",
        watches: &[
            "crates/ferrosift-operations/src/base64",
            "crates/ferrosift-operations/src/hex",
            "crates/ferrosift-operations/src/hex_util.rs",
            "crates/ferrosift-operations/src/alphabet.rs",
            "bench/benches/encoding.rs",
        ],
    },
    Batch {
        name: "digest",
        watches: &[
            "crates/ferrosift-operations/src/checksum",
            "crates/ferrosift-operations/src/sets",
            "bench/benches/digest.rs",
        ],
    },
    Batch {
        name: "dispatch",
        watches: &[
            "crates/ferrosift/src",
            "crates/ferrosift-operations/src/hash",
            "crates/ferrosift-operations/src/identity.rs",
            "crates/ferrosift-operations/src/registry.rs",
            "bench/benches/dispatch.rs",
        ],
    },
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn provenance_dir() -> PathBuf {
    repo_root().join("bench").join("target").join("provenance")
}

/// Runs the named batches, or every batch when none are named.
fn measure(requested: &[&str]) -> ExitCode {
    let selected: Vec<&Batch> = if requested.is_empty() {
        BATCHES.iter().collect()
    } else {
        let mut selected = Vec::new();
        for name in requested {
            match BATCHES.iter().find(|batch| batch.name == *name) {
                Some(batch) => selected.push(batch),
                None => {
                    eprintln!("unknown batch: {name}");
                    eprintln!(
                        "known batches: {}",
                        BATCHES
                            .iter()
                            .map(|batch| batch.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    return ExitCode::FAILURE;
                }
            }
        }
        selected
    };

    let directory = repo_root().join("bench").to_string_lossy().to_string();
    if std::fs::create_dir_all(provenance_dir()).is_err() {
        eprintln!("could not create the provenance directory");
        return ExitCode::FAILURE;
    }

    for batch in selected {
        let arguments = [
            "bench",
            "--bench",
            batch.name,
            "--",
            "--warm-up-time",
            "3",
            "--measurement-time",
            "8",
        ];
        if !run_streaming("cargo", &arguments, Some(&directory)) {
            return ExitCode::FAILURE;
        }
        // Written per batch, after that batch succeeded, so a partial sweep
        // leaves accurate provenance rather than claiming the whole report was
        // measured at this commit.
        if record_provenance(batch.name) == ExitCode::FAILURE {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Re-runs only the batches whose sources have changed.
fn rerun_stale() -> ExitCode {
    let stale = stale_batches();
    if stale.is_empty() {
        println!("every batch is current; nothing to re-run");
        return ExitCode::SUCCESS;
    }
    println!(
        "re-running {} of {} batches: {}",
        stale.len(),
        BATCHES.len(),
        stale.join(", ")
    );
    let names: Vec<&str> = stale.iter().map(String::as_str).collect();
    if measure(&names) == ExitCode::SUCCESS {
        report()
    } else {
        ExitCode::FAILURE
    }
}

/// Records what one batch was measured on.
///
/// A timing without a machine behind it is not evidence of anything, and one
/// without a commit cannot be told apart from a measurement of code that has
/// since been rewritten.
fn record_provenance(batch: &str) -> ExitCode {
    let Some(rustc) = capture("rustc", &["-vV"]) else {
        eprintln!("could not read the compiler version");
        return ExitCode::FAILURE;
    };
    let cpu = std::env::var("PROCESSOR_IDENTIFIER")
        .ok()
        .or_else(read_proc_cpuinfo)
        .unwrap_or_else(|| String::from("unknown"));
    let commit = capture("git", &["rev-parse", "HEAD"])
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| String::from("unknown"));
    let dirty =
        capture("git", &["status", "--porcelain"]).is_some_and(|value| !value.trim().is_empty());

    let escape = |value: &str| value.replace('\\', "\\\\").replace('"', "\\\"");
    let document = format!(
        "{{\n  \"batch\": \"{}\",\n  \"rustc\": \"{}\",\n  \"os\": \"{}\",\n  \"arch\": \"{}\",\n  \"cpu\": \"{}\",\n  \"commit\": \"{}\",\n  \"dirty\": {}\n}}\n",
        escape(batch),
        escape(rustc.trim()).replace('\n', "\\n"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        escape(cpu.trim()),
        escape(&commit),
        dirty,
    );
    let path = provenance_dir().join(format!("{batch}.json"));
    match std::fs::write(&path, document) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("could not write {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}

/// Reports which batches are stale, and fails when any is.
fn check() -> ExitCode {
    let mut any_stale = false;
    for batch in BATCHES {
        match batch_commit(batch.name) {
            None => {
                println!("{:<10} never measured", batch.name);
                any_stale = true;
            }
            Some(commit) => {
                let changed = changed_since(&commit, batch);
                if changed.is_empty() {
                    println!("{:<10} current at {}", batch.name, short(&commit));
                } else {
                    println!(
                        "{:<10} STALE — measured at {}, {} file(s) changed since",
                        batch.name,
                        short(&commit),
                        changed.len()
                    );
                    for file in changed.iter().take(5) {
                        println!("             {file}");
                    }
                    if changed.len() > 5 {
                        println!("             ... and {} more", changed.len() - 5);
                    }
                    any_stale = true;
                }
            }
        }
    }
    if any_stale {
        eprintln!();
        eprintln!("re-run only what changed:  cargo xtask bench stale");
        eprintln!("published numbers describe the code that produced them, not HEAD.");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn stale_batches() -> Vec<String> {
    BATCHES
        .iter()
        .filter(|batch| match batch_commit(batch.name) {
            None => true,
            Some(commit) => !changed_since(&commit, batch).is_empty(),
        })
        .map(|batch| batch.name.to_owned())
        .collect()
}

fn batch_commit(batch: &str) -> Option<String> {
    let document = std::fs::read_to_string(provenance_dir().join(format!("{batch}.json"))).ok()?;
    let commit = field(&document, "commit")?;
    (commit != "unknown").then_some(commit)
}

/// Files under this batch's watch list that changed since it was measured.
fn changed_since(commit: &str, batch: &Batch) -> Vec<String> {
    let mut arguments = vec!["diff", "--name-only", commit, "--"];
    arguments.extend(batch.watches.iter().copied());
    arguments.extend(SHARED.iter().copied());
    capture("git", &arguments)
        .map(|output| {
            output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn short(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

/// Reads one string field out of the small generated JSON document.
///
/// A hand-rolled reader rather than a dependency: xtask has none, and the
/// document is written by this file rather than by a user.
fn field(document: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\"");
    let start = document.find(&key)? + key.len();
    let rest = document.get(start..)?;
    let open = rest.find('"')? + 1;
    let value = rest.get(open..)?;
    let end = value.find('"')?;
    Some(value.get(..end)?.to_owned())
}

fn capture(program: &str, arguments: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn read_proc_cpuinfo() -> Option<String> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    text.lines()
        .find(|line| line.starts_with("model name"))
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().to_owned())
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
