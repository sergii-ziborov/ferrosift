//! How much of the real `.hexpat` ecosystem this subset reads.
//!
//! The differential corpus answers a different question. Its 104 cases
//! separate one construct each, which is what makes a failure attributable —
//! and it says nothing about patterns people actually wrote, because nobody
//! writes one construct at a time. The README admitted that gap in words; this
//! is the number.
//!
//! The subject is `WerWolv/ImHex-Patterns`, the repository `ImHex` itself ships:
//! every `.hexpat` under `patterns/`, parsed, with the verdict recorded per
//! file. A pattern that parses is one this crate could evaluate given the
//! bytes; a pattern that does not names the construct that stopped it, which
//! turns "some of the language is missing" into a ranked list of what.
//!
//! # Why the sources are not here
//!
//! `ImHex-Patterns` is GPL-2.0 and this repository is Apache-2.0, so the
//! patterns are not vendored — the fixture records each file's path, size and
//! content digest instead, and the checkout stays gitignored beside the other
//! reference checkouts. That makes this replayable by anyone at the same
//! commit and keeps the licences apart.
//!
//! Without the checkout the survey cannot run, so this file does two things.
//! It replays the survey where the checkout is present, and it checks the
//! committed fixture against itself everywhere — a published number that
//! disagreed with the list it summarises would be wrong in the tree whether or
//! not anyone could regenerate it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ferrosift_pattern::parse;
use serde::{Deserialize, Serialize};

const FIXTURE: &str = include_str!("fixtures/ecosystem.json");

/// The pinned commit, which the checkout must be on for a replay to mean
/// anything.
const COMMIT: &str = "4b25356eb7bec31ad33d6b196e8173c832b195f1";

#[derive(Deserialize, Serialize)]
struct Survey {
    source: Source,
    summary: Summary,
    /// Failure codes and how many patterns each stopped, most common first.
    ///
    /// The actionable half: a code here with a large count is one construct
    /// standing between this crate and a lot of real patterns.
    refusals: Vec<Refusal>,
    patterns: Vec<Surveyed>,
}

#[derive(Deserialize, Serialize)]
struct Source {
    repository: String,
    commit: String,
    license: String,
}

#[derive(Deserialize, Serialize)]
struct Summary {
    surveyed: usize,
    parsed: usize,
    refused: usize,
}

#[derive(Deserialize, Serialize)]
struct Refusal {
    code: String,
    patterns: usize,
}

#[derive(Deserialize, Serialize)]
struct Surveyed {
    /// Repository-relative path, with forward slashes on every platform.
    path: String,
    bytes: usize,
    /// FNV-1a over the file's bytes.
    ///
    /// Not a cryptographic digest and not trying to be: nothing here is
    /// defending against a forged pattern, only against the fixture and the
    /// checkout drifting apart without anyone noticing. A hash that needs no
    /// dependency is the right size of tool for that.
    digest: String,
    /// `null` where the pattern parsed.
    code: Option<String>,
    /// The line the parser stopped on, where it stopped.
    line: Option<u32>,
}

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/pattern-oracle/vendor/ImHex-Patterns")
}

/// FNV-1a, 64-bit.
fn digest(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Every `.hexpat` under `patterns/`, in a stable order.
fn collect(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.join("patterns")];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "hexpat")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Parses every pattern and records what happened.
fn survey(root: &Path) -> Survey {
    let mut patterns = Vec::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for path in collect(root) {
        let bytes = std::fs::read(&path).expect("a listed file is readable");
        // The parser takes text. A pattern that is not UTF-8 is refused under
        // its own code rather than being skipped, because "this crate cannot
        // read it" is the answer either way.
        let outcome = core::str::from_utf8(&bytes)
            .map_err(|_| (String::from("pattern.source.not_utf8"), None))
            .and_then(|text| {
                parse(text)
                    .map_err(|error| (String::from(error.code()), Some(error.position().line)))
            });

        let relative = path
            .strip_prefix(root)
            .expect("every path came from under the root")
            .to_string_lossy()
            .replace('\\', "/");
        let (code, line) = match outcome {
            Ok(_) => (None, None),
            Err((code, line)) => {
                *counts.entry(code.clone()).or_default() += 1;
                (Some(code), line)
            }
        };
        patterns.push(Surveyed {
            path: relative,
            bytes: bytes.len(),
            digest: digest(&bytes),
            code,
            line,
        });
    }

    let parsed = patterns.iter().filter(|one| one.code.is_none()).count();
    let mut refusals: Vec<Refusal> = counts
        .into_iter()
        .map(|(code, patterns)| Refusal { code, patterns })
        .collect();
    // Most common first, then by code, so the order is total and the ranking
    // is the one a reader wants.
    refusals.sort_by(|a, b| {
        b.patterns
            .cmp(&a.patterns)
            .then_with(|| a.code.cmp(&b.code))
    });

    Survey {
        source: Source {
            repository: String::from("https://github.com/WerWolv/ImHex-Patterns"),
            commit: String::from(COMMIT),
            license: String::from("GPL-2.0"),
        },
        summary: Summary {
            surveyed: patterns.len(),
            parsed,
            refused: patterns.len() - parsed,
        },
        refusals,
        patterns,
    }
}

/// The committed survey must add up.
///
/// Runs everywhere, checkout or not. A published number that disagreed with
/// the list it summarises would be wrong in the tree whether or not anyone
/// could regenerate it — and this file's whole purpose is to publish a number.
#[test]
fn the_recorded_survey_is_internally_consistent() {
    let recorded: Survey = serde_json::from_str(FIXTURE).expect("the survey fixture parses");

    assert_eq!(recorded.source.commit, COMMIT);
    assert_eq!(recorded.source.license, "GPL-2.0");
    assert_eq!(recorded.summary.surveyed, recorded.patterns.len());
    assert_eq!(
        recorded.summary.parsed + recorded.summary.refused,
        recorded.summary.surveyed
    );
    assert_eq!(
        recorded
            .patterns
            .iter()
            .filter(|one| one.code.is_none())
            .count(),
        recorded.summary.parsed
    );

    // Every refusal in the ranking is one the list actually holds, with the
    // count the list actually gives.
    let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
    for pattern in &recorded.patterns {
        if let Some(code) = &pattern.code {
            *counted.entry(code.as_str()).or_default() += 1;
        }
    }
    assert_eq!(counted.len(), recorded.refusals.len());
    for refusal in &recorded.refusals {
        assert_eq!(
            counted.get(refusal.code.as_str()).copied(),
            Some(refusal.patterns),
            "{} is ranked with a count the list does not support",
            refusal.code
        );
    }

    // A floor, so a fixture that silently lost most of its entries fails
    // rather than passing with a smaller subject.
    assert!(
        recorded.summary.surveyed >= 250,
        "the survey looks truncated: {} patterns",
        recorded.summary.surveyed
    );
    assert!(recorded.summary.parsed > 0, "no pattern parsed at all");
}

/// Re-runs the survey where the checkout is present, and requires the same
/// answers.
///
/// Skipped rather than failed without it: the patterns are GPL-2.0 and cannot
/// live in this tree, so an ordinary `cargo test` has nothing to read. The
/// weekly reference job has the checkout and is where this actually bites.
#[test]
fn the_survey_still_says_what_was_recorded() {
    let root = checkout();
    if !root.join("patterns").is_dir() {
        eprintln!(
            "skipping: no ImHex-Patterns checkout at {}\n\
             run: cargo xtask pattern survey",
            root.display()
        );
        return;
    }

    let recorded: Survey = serde_json::from_str(FIXTURE).expect("the survey fixture parses");
    let fresh = survey(&root);

    if std::env::var_os("FERROSIFT_RECORD_SURVEY").is_some() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ecosystem.json");
        let json = serde_json::to_string_pretty(&fresh).expect("the survey serializes");
        std::fs::write(&path, format!("{json}\n")).expect("the fixture is writable");
        eprintln!(
            "recorded {} patterns, {} parsed, to {}",
            fresh.summary.surveyed,
            fresh.summary.parsed,
            path.display()
        );
        return;
    }

    assert_eq!(
        fresh.summary.surveyed, recorded.summary.surveyed,
        "the checkout holds a different number of patterns than the fixture records; \
         is it on commit {COMMIT}?"
    );
    assert_eq!(
        fresh.summary.parsed, recorded.summary.parsed,
        "the parse rate moved: {} now, {} recorded",
        fresh.summary.parsed, recorded.summary.parsed
    );

    for (fresh, recorded) in fresh.patterns.iter().zip(&recorded.patterns) {
        assert_eq!(fresh.path, recorded.path, "the pattern list changed order");
        assert_eq!(
            fresh.digest, recorded.digest,
            "{} has different contents than the fixture was built from",
            fresh.path
        );
        assert_eq!(
            fresh.code, recorded.code,
            "{} is answered differently than recorded",
            fresh.path
        );
    }
}
