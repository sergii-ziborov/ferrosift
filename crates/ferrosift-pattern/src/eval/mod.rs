mod composite;
mod evaluator;
mod expression;
mod options;
mod reader;
mod value;

pub use evaluator::evaluate;
pub use options::EvalOptions;
pub use value::{Node, NodeValue};

pub(crate) use expression::fold;
