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
            "3",
            "--measurement-time",
            "8",
        ];
        if !run_streaming("cargo", &arguments, Some(&directory)) {
            return ExitCode::FAILURE;
        }
    }
    record_environment(&directory)
}

/// Records what the numbers were measured on.
///
/// A timing without a machine behind it is not evidence of anything, and a
/// reader cannot tell a real win from a lucky one without knowing the
/// compiler and the CPU. This is written beside the raw results so the report
/// can state both.
fn record_environment(directory: &str) -> ExitCode {
    let Some(rustc) = capture("rustc", &["-vV"]) else {
        eprintln!("could not read the compiler version");
        return ExitCode::FAILURE;
    };
    let cpu = std::env::var("PROCESSOR_IDENTIFIER")
        .ok()
        .or_else(read_proc_cpuinfo)
        .unwrap_or_else(|| String::from("unknown"));

    let escape = |value: &str| value.replace('\\', "\\\\").replace('"', "\\\"");
    let document = format!(
        "{{\n  \"rustc\": \"{}\",\n  \"os\": \"{}\",\n  \"arch\": \"{}\",\n  \"cpu\": \"{}\"\n}}\n",
        escape(rustc.trim()).replace('\n', "\\n"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        escape(cpu.trim()),
    );
    let path = Path::new(directory).join("target").join("environment.json");
    match std::fs::write(&path, document) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("could not write {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
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
