use crate::{DisplayUnit, Interval, ValueType};

use super::ExpectedType;

/// Errors that can occur when evaluating a binary operation.
///
/// Note that all `ValueType`s are boxed to decrease the size
/// of the error enum.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryEvalError {
    /// Unit mismatch between operands.
    UnitMismatch {
        /// Unit of the left-hand side.
        lhs_unit: DisplayUnit,
        /// Unit of the right-hand side.
        rhs_unit: DisplayUnit,
    },
    /// Type mismatch between operands.
    TypeMismatch {
        /// Type of the left-hand side.
        lhs_type: Box<ValueType>,
        /// Type of the right-hand side.
        rhs_type: Box<ValueType>,
    },
    /// Left-hand side has an invalid type.
    InvalidLhsType {
        /// Type that was expected for the left-hand side.
        expected_type: ExpectedType,
        /// Actual type of the left-hand side.
        lhs_type: Box<ValueType>,
    },
    /// Right-hand side has an invalid type.
    InvalidRhsType {
        /// Type that was expected for the right-hand side.
        expected_type: ExpectedType,
        /// Actual type of the right-hand side.
        rhs_type: Box<ValueType>,
    },
    /// Exponent has units (not allowed).
    ExponentHasUnits {
        /// Unit of the exponent (must be unitless).
        exponent_unit: DisplayUnit,
    },
    /// Exponent is an interval (not allowed when base has unit).
    ExponentIsInterval {
        /// Interval used as exponent (must be scalar when base has unit).
        exponent_interval: Interval,
    },
}
