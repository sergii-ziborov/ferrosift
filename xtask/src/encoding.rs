//! Guards the repository against double-encoded text.
//!
//! This exists because the mistake has been made twice. A read-modify-write
//! through PowerShell 5.1 reads a UTF-8 file as the ANSI code page and writes
//! it back as UTF-8, so every non-ASCII character becomes the two or three
//! characters its bytes look like in CP-1252 — an em dash turns into `â€”` and
//! stays that way, in committed source, unnoticed, because nothing compiles
//! differently and no test reads comments.
//!
//! One pass over the tracked text files is cheap enough to run in CI, and
//! turns a class of damage that is invisible in review into a failing build.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Extensions worth scanning: everything a human writes prose in.
const EXTENSIONS: &[&str] = &["rs", "mjs", "js", "md", "toml", "yml", "yaml"];

/// Directories that are not ours to police.
const SKIP: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".ferrosift-private",
    "vendor",
];

/// This scanner's own path, relative to the repository root.
const SELF: &str = "xtask/src/encoding.rs";

/// Sequences that only arise from double-encoding, and what they should be.
///
/// Deliberately not "any run of high characters". Real prose in this repository
/// contains accented text on purpose — the corpus builders sample `éè` and
/// `ÄÖ` precisely because those distinguish one encoding from another — and a
/// heuristic that flagged those would be turned off within a week. Every entry
/// here is a sequence with no meaning as text.
const DOUBLE_ENCODED: &[(&str, &str)] = &[
    ("â€”", "—"),
    ("â€“", "–"),
    ("â€™", "’"),
    ("â€˜", "‘"),
    ("â€œ", "“"),
    ("â€¦", "…"),
    ("Ã©", "é"),
    ("Ã¨", "è"),
    ("Ã¼", "ü"),
    ("Ã¶", "ö"),
    ("Ã¤", "ä"),
    ("Ã ", "à"),
    ("Ã§", "ç"),
    ("Ã±", "ñ"),
    ("Ã¿", "ÿ"),
    ("Â ", "\u{a0}"),
    ("Â«", "«"),
    ("Â»", "»"),
    // Triple-encoded, which is what happens when the same file goes through
    // twice. Listed separately so the report names what it actually found.
    ("Ã¢â‚¬", "—"),
];

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["check"] | [] => check(),
        other => {
            eprintln!("unknown encoding task: {}", other.join(" "));
            eprintln!("expected: check");
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Reports every file damaged by an encoding round trip.
fn check() -> ExitCode {
    let mut findings: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    walk(&repo_root(), &mut scanned, &mut findings);

    if findings.is_empty() {
        println!("no encoding damage in {scanned} files");
        return ExitCode::SUCCESS;
    }

    eprintln!("encoding damage in {} place(s):", findings.len());
    for finding in &findings {
        eprintln!("  {finding}");
    }
    eprintln!(
        "\nBoth failures come from the same place: a PowerShell\n\
         `Get-Content | Set-Content` round trip. Mojibake is UTF-8 that was\n\
         read as CP-1252 and written back as UTF-8; a byte-order mark is what\n\
         `Set-Content -Encoding utf8` adds on Windows PowerShell whether or not\n\
         the file had one. Edit with a tool that preserves the encoding."
    );
    ExitCode::FAILURE
}

fn walk(directory: &Path, scanned: &mut usize, findings: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIP.contains(&name.as_ref()) {
            continue;
        }
        if path.is_dir() {
            walk(&path, scanned, findings);
            continue;
        }
        let extension = path.extension().unwrap_or_default().to_string_lossy();
        if !EXTENSIONS.contains(&extension.as_ref()) {
            continue;
        }
        // This file *is* the table of broken sequences, so it matches every
        // pattern by construction. Skipping it is not an exemption carved out
        // for convenience: there is nothing here to detect.
        if path.ends_with(SELF) {
            continue;
        }
        *scanned += 1;
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        inspect(&path, &text, findings);
    }
}

fn inspect(path: &Path, text: &str, findings: &mut Vec<String>) {
    // A byte-order mark is invisible in an editor and legal UTF-8, so nothing
    // else here would report it -- but it is the first byte of the file, so it
    // lands inside the opening token. In a Rust file that is a `//!` that no
    // longer starts a doc comment; in a shell script it is text before the
    // shebang. Windows tooling writes one by default, which is how two of them
    // reached this tree.
    if text.starts_with('\u{feff}') {
        findings.push(format!(
            "{}:1  file begins with a byte-order mark",
            path.display()
        ));
    }
    for (number, line) in text.lines().enumerate() {
        for (broken, fixed) in DOUBLE_ENCODED {
            if line.contains(broken) {
                findings.push(format!(
                    "{}:{}  {broken:?} should be {fixed:?}",
                    path.display(),
                    number + 1
                ));
            }
        }
    }
}
