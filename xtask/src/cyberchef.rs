//! Reference-oracle tasks.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::{capture, run_streaming};

/// One pinned reference version.
///
/// More than one, because a compatibility claim is against a version rather
/// than against a project. Evidence for an older profile is kept when a newer
/// one is added: a caller pinned to 11.3 is entitled to know FerroSift still
/// matches it.
struct Profile {
    version: &'static str,
    commit: &'static str,
}

const PROFILES: &[Profile] = &[
    Profile {
        version: "11.3.0",
        commit: "d24ba1afce2e3a080308b5df7db033332fe94a1a",
    },
    Profile {
        version: "11.4.0",
        // The annotated tag points at this commit; the tag object's own SHA
        // would check out nothing useful.
        commit: "49d1a5634a67a3b806c6db0fdca7dcecb41a776c",
    },
];

/// Used when `--profile` is not given.
const DEFAULT_PROFILE: &str = "11.3.0";

const UPSTREAM: &str = "https://github.com/gchq/CyberChef.git";

/// Reads `--profile <version>` out of the arguments.
///
/// An unknown name is refused rather than defaulted: measuring against a
/// different version than the one asked for is the failure this arrangement
/// exists to prevent.
fn profile_from(arguments: &[&str]) -> Result<&'static Profile, String> {
    let requested = arguments
        .iter()
        .position(|value| *value == "--profile")
        .and_then(|index| arguments.get(index + 1).copied())
        .unwrap_or(DEFAULT_PROFILE);

    PROFILES
        .iter()
        .find(|profile| profile.version == requested)
        .ok_or_else(|| {
            let known: Vec<&str> = PROFILES.iter().map(|p| p.version).collect();
            format!("unknown profile {requested}; known: {}", known.join(", "))
        })
}

pub fn run(arguments: &[&str]) -> ExitCode {
    match arguments {
        ["setup", rest @ ..] => dispatch(rest, setup),
        ["generate", rest @ ..] => dispatch(rest, generate),
        ["verify", rest @ ..] => dispatch(rest, verify),
        ["overlay", rest @ ..] => dispatch(rest, overlay),
        ["gap", rest @ ..] => gap(rest),
        other => {
            eprintln!("unknown cyberchef task: {}", other.join(" "));
            ExitCode::FAILURE
        }
    }
}

/// Resolves the profile, then runs the task with it.
fn dispatch(arguments: &[&str], task: fn(&Profile) -> ExitCode) -> ExitCode {
    match profile_from(arguments) {
        Ok(profile) => task(profile),
        Err(message) => {
            eprintln!("{message}");
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
fn checkout_dir(profile: &Profile) -> PathBuf {
    let version = profile.version;
    std::env::var_os("FERROSIFT_CYBERCHEF_DIR").map_or_else(
        || {
            oracle_dir()
                .join("vendor")
                .join(format!("cyberchef-v{version}"))
        },
        PathBuf::from,
    )
}

/// Clones the reference at the pinned commit and installs its dependencies.
///
/// The checkout is never committed: it is large, it is upstream's, and pinning
/// it by commit makes re-fetching reproducible.
fn setup(profile: &Profile) -> ExitCode {
    let checkout = checkout_dir(profile);
    if checkout.exists() {
        eprintln!("checkout already present at {}", checkout.display());
        return verify(profile);
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
    // `npx grunt node` writes src/node/index.mjs, the barrel the oracle
    // imports. It is generated rather than committed, so a fresh clone has no
    // entry point until this runs.
    let steps: [(&str, Vec<&str>, Option<&str>); 4] = [
        (
            "git",
            vec!["clone", "--no-checkout", UPSTREAM, &target],
            None,
        ),
        ("git", vec!["-C", &target, "checkout", profile.commit], None),
        ("npm", vec!["ci"], Some(&target)),
        ("npx", vec!["grunt", "node"], Some(&target)),
    ];
    for (program, arguments, directory) in steps {
        if !run_streaming(program, &arguments, directory) {
            return ExitCode::FAILURE;
        }
    }
    verify(profile)
}

/// Confirms the checkout sits exactly on the pinned commit, then replays it.
///
/// The commit is compared rather than printed. It used to be printed beside
/// the expected one and left for a reader to check, which works at a terminal
/// and does nothing at all in a scheduled job — a checkout left on some other
/// revision would have passed silently, which is the one failure this whole
/// arrangement exists to prevent.
fn verify(profile: &Profile) -> ExitCode {
    let checkout = checkout_dir(profile);
    if !checkout.exists() {
        eprintln!(
            "reference checkout missing at {}\nrun: cargo xtask cyberchef setup --profile {}",
            checkout.display(),
            profile.version
        );
        return ExitCode::FAILURE;
    }
    let target = checkout.to_string_lossy().to_string();
    let Some(head) = capture("git", &["-C", &target, "rev-parse", "HEAD"]) else {
        eprintln!("cannot read the checkout's HEAD at {target}");
        return ExitCode::FAILURE;
    };
    if head.trim() != profile.commit {
        eprintln!(
            "checkout at {target} is on {}, not the pinned v{} commit {}",
            head.trim(),
            profile.version,
            profile.commit
        );
        return ExitCode::FAILURE;
    }
    eprintln!("v{} is at {}", profile.version, profile.commit);

    // Replay the fixtures this profile is actually recorded in: the baseline
    // is stored in full and everything after it as an overlay, so the two are
    // different test targets and running the baseline's for a later profile
    // would report on a version nobody asked about.
    let replays: &[&str] = if profile.version == DEFAULT_PROFILE {
        &["corpus", "differential", "flow"]
    } else {
        &["profiles"]
    };
    let root = repo_root().to_string_lossy().to_string();
    for replay in replays {
        if !run_streaming(
            "cargo",
            &["test", "-p", "ferrosift-operations", "--test", replay],
            Some(&root),
        ) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Regenerates both pinned fixtures from the reference.
fn generate(profile: &Profile) -> ExitCode {
    for script in ["generate-suite.mjs", "generate-corpus.mjs", "generate-flow.mjs"] {
        let path = oracle_dir().join(script).to_string_lossy().to_string();
        // The scripts read the same flag, so the profile travels with the
        // request rather than being implied by an environment variable
        // somebody forgot to set.
        if !run_streaming("node", &[&path, "--profile", profile.version], None) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Condenses a non-baseline profile's fixtures into a delta against the baseline.
///
/// Two agreeing profiles would otherwise be stored as two identical
/// million-byte files. What a second profile actually contributes is where it
/// differs, so that is what gets committed; the full generated files stay out
/// of the tree.
fn overlay(profile: &Profile) -> ExitCode {
    let path = oracle_dir()
        .join("overlay.mjs")
        .to_string_lossy()
        .to_string();
    if run_streaming("node", &[&path, "--profile", profile.version], None) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
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
