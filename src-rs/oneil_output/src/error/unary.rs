use crate::ValueType;

/// Errors that can occur when evaluating a unary operation.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryEvalError {
    /// Negation was applied to a value whose type does not support it.
    InvalidNegType {
        /// Actual type of the value.
        value_type: Box<ValueType>,
    },
    /// Logical not was applied to a value whose type does not support it.
    InvalidNotType {
        /// Actual type of the value.
        value_type: Box<ValueType>,
    },
}
