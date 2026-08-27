//! Development tasks for the `FerroSift` workspace.
//!
//! This crate is deliberately outside the workspace and outside every library
//! dependency graph: nothing it does is reachable from the shipped crates. It
//! exists so the reference oracle is reproducible by anyone, not only by
//! whoever happens to have a checkout lying around.

use std::process::{Command, ExitCode};

mod bench;
mod coverage;
mod cyberchef;
mod encoding;
mod jscompat;
mod ledger;

const USAGE: &str = "\
FerroSift development tasks

Usage:
  cargo xtask cyberchef setup      Clone the pinned reference checkout
  cargo xtask cyberchef generate   Regenerate the pinned fixtures
  cargo xtask cyberchef verify     Check the pin, then replay the fixtures
  cargo xtask cyberchef gap        List reference operations not yet implemented
  cargo xtask ledger generate      Rewrite the derived compatibility ledger
  cargo xtask ledger check         Fail when the committed ledger is stale
  cargo xtask encoding check       Fail on text that was double-encoded
  cargo xtask coverage check       Fail when coverage fell below the floor
  cargo xtask coverage record      Raise the floor to what the suite reaches now

The reference checkout defaults to tools/cyberchef-oracle/vendor and can be
pointed elsewhere with FERROSIFT_CYBERCHEF_DIR.
";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let parts: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match parts.as_slice() {
        ["bench", rest @ ..] => bench::run(rest),
        ["coverage", rest @ ..] => coverage::run(rest),
        ["cyberchef", rest @ ..] => cyberchef::run(rest),
        ["encoding", rest @ ..] => encoding::run(rest),
        ["jscompat", rest @ ..] => jscompat::run(rest),
        ["ledger", rest @ ..] => ledger::run(rest),
        ["--help" | "-h"] | [] => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown task: {}\n", other.join(" "));
            print!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Runs a command, streaming its output, and reports whether it succeeded.
fn run_streaming(program: &str, arguments: &[&str], directory: Option<&str>) -> bool {
    eprintln!("$ {program} {}", arguments.join(" "));
    match spawn(program, arguments, directory) {
        Ok(status) => status.success(),
        Err(error) => {
            eprintln!("failed to run {program}: {error}");
            false
        }
    }
}

/// Starts a program, falling back to the shell for Windows batch shims.
///
/// `npm` and `npx` on Windows are `npm.cmd` and `npx.cmd`. `Command::new`
/// resolves neither: it looks for an executable image and does not consult
/// `PATHEXT`, so a machine with a working npm still reports "program not
/// found". Retrying through `cmd /c` is the documented way to reach a shim.
///
/// The retry is deliberately conditional on the direct spawn failing to find
/// the program, so `git` and `node` — real executables — keep their exact
/// argument vector rather than being re-parsed by the shell.
fn spawn(
    program: &str,
    arguments: &[&str],
    directory: Option<&str>,
) -> std::io::Result<std::process::ExitStatus> {
    let mut command = Command::new(program);
    command.args(arguments);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let error = match command.status() {
        Ok(status) => return Ok(status),
        Err(error) => error,
    };
    if !cfg!(windows) || error.kind() != std::io::ErrorKind::NotFound {
        return Err(error);
    }
    let mut shell = Command::new("cmd");
    shell.arg("/c").arg(program).args(arguments);
    if let Some(directory) = directory {
        shell.current_dir(directory);
    }
    shell.status()
}
