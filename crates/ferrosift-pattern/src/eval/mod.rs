mod composite;
mod evaluator;
mod expression;
mod options;
mod reader;
mod source;
mod value;

pub use evaluator::{evaluate, evaluate_with};
pub use options::EvalOptions;
pub use source::{ByteSource, MAX_SCALAR_BYTES, SourceError};
pub use value::{Node, NodeValue, ScalarArray};

pub(crate) use expression::fold;
