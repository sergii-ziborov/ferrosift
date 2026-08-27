//! A coverage floor that can only be raised.
//!
//! Not a target. A number a build must reach invites tests written to reach it,
//! and a test written for a number checks the number rather than the code. What
//! this holds is the opposite: whatever coverage the suite already has, it does
//! not get to lose. Adding an untested branch is then a visible act — the floor
//! says no, and raising it is a line in a commit somebody has to justify.
//!
//! The slack absorbs measurement noise rather than negligence. Region counts
//! move slightly with inlining decisions and with which tests a run happens to
//! schedule; half a point is wider than that drift and far narrower than a
//! module arriving without tests.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::run_streaming;

/// How far below the recorded floor a run may land before it fails.
const SLACK: f64 = 0.5;

/// Where the floor lives, beside the other generated compatibility records.
fn baseline_path() -> PathBuf {
    repo_root().join("docs/compatibility/coverage.json")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// The three percentages the report carries.
#[derive(Clone, Copy, Debug, Default)]
struct Coverage {
    regions: f64,
    functions: f64,
    lines: f64,
}

pub(crate) fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["check"] => check(),
        ["record"] => record(),
        other => {
            eprintln!(
                "unknown coverage task: {}\n\
                 \n\
                 Usage:\n\
                 \x20 cargo xtask coverage check   Fail when coverage fell below the floor\n\
                 \x20 cargo xtask coverage record  Raise the floor to what the suite reaches now\n",
                other.join(" ")
            );
            ExitCode::FAILURE
        }
    }
}

/// Measures, compares against the floor, and reports both.
fn check() -> ExitCode {
    let Some(measured) = measure() else {
        return ExitCode::FAILURE;
    };
    let Some(floor) = read_baseline() else {
        eprintln!(
            "no coverage floor recorded yet; run `cargo xtask coverage record` and commit it"
        );
        return ExitCode::FAILURE;
    };

    let mut failures = Vec::new();
    for (name, now, then) in [
        ("regions", measured.regions, floor.regions),
        ("functions", measured.functions, floor.functions),
        ("lines", measured.lines, floor.lines),
    ] {
        if now < then - SLACK {
            failures.push(format!("{name}: {now:.2}% against a floor of {then:.2}%"));
        }
    }

    println!(
        "coverage: {:.2}% regions, {:.2}% functions, {:.2}% lines \
         (floor {:.2} / {:.2} / {:.2}, slack {SLACK})",
        measured.regions,
        measured.functions,
        measured.lines,
        floor.regions,
        floor.functions,
        floor.lines,
    );

    if failures.is_empty() {
        return ExitCode::SUCCESS;
    }
    eprintln!("coverage fell below the recorded floor:");
    for failure in &failures {
        eprintln!("  {failure}");
    }
    eprintln!(
        "\nAdd tests for what the change introduced, or -- if the drop is real and \
         accepted -- run `cargo xtask coverage record` and say why in the commit."
    );
    ExitCode::FAILURE
}

/// Writes what the suite reaches now as the new floor.
fn record() -> ExitCode {
    let Some(measured) = measure() else {
        return ExitCode::FAILURE;
    };
    let body = format!(
        "{{\n  \"comment\": [\n\
         \x20   \"The coverage floor, not a target. `cargo xtask coverage check` fails when a\",\n\
         \x20   \"run lands more than half a point under any of these. Raising it is deliberate:\",\n\
         \x20   \"run `cargo xtask coverage record` and say in the commit what was added.\"\n\
         \x20 ],\n\
         \x20 \"regions\": {:.2},\n\
         \x20 \"functions\": {:.2},\n\
         \x20 \"lines\": {:.2}\n}}\n",
        measured.regions, measured.functions, measured.lines
    );
    if let Err(error) = fs::write(baseline_path(), body) {
        eprintln!("could not write the coverage floor: {error}");
        return ExitCode::FAILURE;
    }
    println!(
        "recorded {:.2}% regions, {:.2}% functions, {:.2}% lines",
        measured.regions, measured.functions, measured.lines
    );
    ExitCode::SUCCESS
}

/// Runs `cargo llvm-cov` and reads the three totals out of its JSON.
fn measure() -> Option<Coverage> {
    let report = repo_root().join("target/coverage-summary.json");
    let path = report.to_string_lossy().to_string();
    if !run_streaming(
        "cargo",
        &[
            "llvm-cov",
            "--workspace",
            "--all-features",
            "--summary-only",
            "--json",
            "--output-path",
            &path,
        ],
        Some(&repo_root().to_string_lossy()),
    ) {
        eprintln!(
            "cargo llvm-cov failed; install it with `cargo install cargo-llvm-cov` if missing"
        );
        return None;
    }

    let raw = fs::read_to_string(&report)
        .map_err(|error| eprintln!("could not read {path}: {error}"))
        .ok()?;
    parse(&raw)
}

/// Pulls the three percentages out of the report.
///
/// Read by hand rather than through a JSON crate, because this is the only
/// place in the task runner that reads JSON at all and the shape is fixed:
/// llvm-cov writes one `totals` object whose members each carry a `percent`.
/// A dependency for one field would be paid for by every build of this crate.
fn parse(raw: &str) -> Option<Coverage> {
    let totals = raw.rfind("\"totals\":").map(|at| &raw[at..])?;
    Some(Coverage {
        regions: percent_after(totals, "\"regions\":")?,
        functions: percent_after(totals, "\"functions\":")?,
        lines: percent_after(totals, "\"lines\":")?,
    })
}

/// The first `"percent"` following `key`.
fn percent_after(text: &str, key: &str) -> Option<f64> {
    let section = text.find(key).map(|at| &text[at..])?;
    let value = section.find("\"percent\":").map(|at| &section[at + 10..])?;
    let end = value
        .find(|character: char| !matches!(character, '0'..='9' | '.' | '-' | '+' | 'e' | 'E'))
        .unwrap_or(value.len());
    value[..end].trim().parse().ok()
}

/// Reads the recorded floor, or nothing when it has never been written.
fn read_baseline() -> Option<Coverage> {
    let raw = fs::read_to_string(baseline_path()).ok()?;
    Some(Coverage {
        regions: number_after(&raw, "\"regions\":")?,
        functions: number_after(&raw, "\"functions\":")?,
        lines: number_after(&raw, "\"lines\":")?,
    })
}

/// The number following `key`.
fn number_after(text: &str, key: &str) -> Option<f64> {
    let value = text.find(key).map(|at| &text[at + key.len()..])?;
    let trimmed = value.trim_start();
    let end = trimmed
        .find(|character: char| !matches!(character, '0'..='9' | '.' | '-'))
        .unwrap_or(trimmed.len());
    trimmed[..end].parse().ok()
}
