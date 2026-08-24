use ferrosift_model::{TextEncoding, TextValue, Value};
use serde::Deserialize;

const FIXTURE: &str = include_str!("../../fixtures/cyberchef-v11.3.0/differential.json");
const CORPUS: &str = include_str!("../../fixtures/cyberchef-v11.3.0/corpus.json");

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

#[derive(Debug, Deserialize)]
pub struct Case {
    pub name: String,
    pub input: Input,
    pub recipe: Vec<serde_json::Value>,
    pub outputs_hex: Vec<String>,
    pub stopped_after: usize,
}

#[derive(Debug, Deserialize)]
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
