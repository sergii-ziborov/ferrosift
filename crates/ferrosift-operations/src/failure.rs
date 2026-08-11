use ferrosift_core::{OperationError, OperationFailureCode};

pub(crate) fn failed(value: &'static str) -> OperationError {
    OperationError::Failed {
        code: OperationFailureCode::from_static(value),
    }
}
