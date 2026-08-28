use std::collections::{BTreeMap, BTreeSet};

use ferrosift_model::{TextEncoding, TextValue, Value};
use serde::Deserialize;

const FIXTURE: &str = include_str!("../../fixtures/cyberchef-v11.3.0/differential.json");
const CORPUS: &str = include_str!("../../fixtures/cyberchef-v11.3.0/corpus.json");

/// Flow control, baked through the reference's own `Recipe` interpreter.
///
/// A separate file because it is produced a different way. The Node API the
/// rest of the corpus goes through refuses these operations outright, so the
/// generator drives `Recipe.execute` directly — the same code path the browser
/// uses, from the same pinned commit.
const FLOW: &str = include_str!("../../fixtures/cyberchef-v11.3.0/flow.json");

/// Deltas that turn the baseline fixtures into a later profile's.
///
/// A second profile that agrees everywhere would otherwise be a second
/// identical million-byte file, and a third would be a third. What a later
/// profile actually contributes is where it differs, so only that is stored;
/// [`apply_overlay`] rebuilds the full profile from the pair.
const CORPUS_11_4: &str = include_str!("../../fixtures/cyberchef-v11.4.0/corpus.overlay.json");
const FIXTURE_11_4: &str =
    include_str!("../../fixtures/cyberchef-v11.4.0/differential.overlay.json");
const FLOW_11_4: &str = include_str!("../../fixtures/cyberchef-v11.4.0/flow.overlay.json");

#[derive(Debug, Deserialize)]
pub struct Suite {
    pub reference: Reference,
    pub cases: Vec<Case>,
    pub unsupported: UnsupportedCase,
}

#[derive(Debug, Deserialize)]
pub struct CorpusSuite {
    pub reference: Reference,
    pub cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
pub struct Reference {
    pub name: String,
    pub version: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Case {
    pub name: String,
    pub input: Input,
    pub recipe: Vec<serde_json::Value>,
    pub outputs_hex: Vec<String>,
    pub stopped_after: usize,
}

/// A later profile expressed as a delta against the baseline.
#[derive(Debug, Deserialize)]
pub struct Overlay {
    pub reference: Reference,
    pub baseline: OverlayBaseline,
    /// How many cases the compared profile had in total.
    ///
    /// Recorded so a reader knows how much agreement empty lists stand for,
    /// and checked after reconstruction so a truncated overlay is caught.
    pub compared_cases: usize,
    /// Cases present in both profiles whose reference output changed.
    pub changed: Vec<Case>,
    /// Cases the later profile has and the baseline does not.
    pub added: Vec<Case>,
    /// Case names the later profile no longer has.
    pub removed: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct OverlayBaseline {
    pub version: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Input {
    Bytes { hex: String },
    Text { value: String },
}

#[derive(Debug, Deserialize)]
pub struct UnsupportedCase {
    pub name: String,
    pub recipe: Vec<serde_json::Value>,
    pub finding: ExpectedFinding,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedFinding {
    pub code: String,
    pub source_step: usize,
    pub original_operation: String,
}

impl Input {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Bytes { hex } => Value::Bytes(decode_hex(hex)),
            Self::Text { value } => Value::Text(TextValue {
                text: value.clone(),
                encoding: TextEncoding::Utf8,
            }),
        }
    }
}

pub fn load_suite() -> Suite {
    serde_json::from_str(FIXTURE).expect("reference fixture must be valid")
}

pub fn load_corpus() -> CorpusSuite {
    serde_json::from_str(CORPUS).expect("corpus fixture must be valid")
}

pub fn load_flow() -> CorpusSuite {
    serde_json::from_str(FLOW).expect("flow fixture must be valid")
}

/// The 11.4 corpus overlay.
pub fn load_corpus_overlay_11_4() -> Overlay {
    serde_json::from_str(CORPUS_11_4).expect("11.4 corpus overlay must be valid")
}

/// The 11.4 flow-control overlay.
pub fn load_flow_overlay_11_4() -> Overlay {
    serde_json::from_str(FLOW_11_4).expect("11.4 flow overlay must be valid")
}

/// The 11.4 differential-suite overlay.
pub fn load_suite_overlay_11_4() -> Overlay {
    serde_json::from_str(FIXTURE_11_4).expect("11.4 differential overlay must be valid")
}

/// Rebuilds a later profile's cases from the baseline and a delta.
///
/// The result is the later profile's own recorded bytes, not an argument that
/// the two profiles are equal: a changed case carries 11.4's output and is
/// replayed as such. Ordering follows the baseline so a failure reports the
/// same case index a reader would find by hand.
///
/// # Panics
///
/// If the overlay disagrees with the baseline about which cases exist, which
/// means one of the two was regenerated without the other.
pub fn apply_overlay(baseline: &[Case], overlay: &Overlay) -> Vec<Case> {
    let mut replacements: BTreeMap<&str, &Case> = BTreeMap::new();
    for case in &overlay.changed {
        assert!(
            baseline.iter().any(|one| one.name == case.name),
            "overlay changes `{}`, which the baseline does not contain; \
             regenerate the overlay against the current baseline",
            case.name
        );
        replacements.insert(case.name.as_str(), case);
    }
    let removed: BTreeSet<&str> = overlay.removed.iter().map(String::as_str).collect();
    for name in &removed {
        assert!(
            baseline.iter().any(|one| one.name == *name),
            "overlay removes `{name}`, which the baseline does not contain; \
             regenerate the overlay against the current baseline"
        );
    }

    let mut cases: Vec<Case> = baseline
        .iter()
        .filter(|case| !removed.contains(case.name.as_str()))
        .map(|case| {
            replacements
                .get(case.name.as_str())
                .map_or_else(|| case.clone(), |replacement| (*replacement).clone())
        })
        .collect();
    cases.extend(overlay.added.iter().cloned());

    assert_eq!(
        cases.len(),
        overlay.compared_cases,
        "reconstructed {} cases but the overlay recorded {} for {}; \
         the overlay is stale or truncated",
        cases.len(),
        overlay.compared_cases,
        overlay.reference.version
    );
    cases
}

impl Case {
    /// The exact `CyberChef` operation names this case exercises, in order.
    pub fn operations(&self) -> Vec<&str> {
        self.recipe
            .iter()
            .filter_map(|step| step.get("op").and_then(serde_json::Value::as_str))
            .collect()
    }
}

pub fn decode_hex(value: &str) -> Vec<u8> {
    assert!(
        value.len().is_multiple_of(2),
        "hex must contain whole bytes"
    );
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = core::str::from_utf8(pair).expect("hex must be ASCII");
            u8::from_str_radix(pair, 16).expect("hex must contain only hexadecimal digits")
        })
        .collect()
}
