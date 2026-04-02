//! Conversion from value-level errors (`BinaryEvalError`, `UnaryEvalError`) to `EvalError`.

use oneil_shared::span::Span;

use super::{BinaryEvalError, EvalError, ExpectedType, UnaryEvalError};

/// Converts a binary eval error from the output crate into an evaluator error.
#[must_use]
pub fn binary_eval_error_to_eval_error(
    error: BinaryEvalError,
    lhs_span: Span,
    rhs_span: Span,
) -> EvalError {
    match error {
        BinaryEvalError::UnitMismatch { lhs_unit, rhs_unit } => EvalError::UnitMismatch {
            expected_unit: lhs_unit,
            expected_source_span: lhs_span,
            found_unit: rhs_unit,
            found_span: rhs_span,
        },
        BinaryEvalError::TypeMismatch {
            expected_type_from_lhs,
            rhs_type,
        } => EvalError::TypeMismatch {
            expected_type: expected_type_from_lhs,
            expected_source_span: lhs_span,
            found_type: *rhs_type,
            found_span: rhs_span,
        },
        BinaryEvalError::InvalidLhsType {
            expected_type,
            lhs_type,
        } => EvalError::InvalidType {
            expected_type,
            found_type: *lhs_type,
            found_span: lhs_span,
        },
        BinaryEvalError::InvalidRhsType {
            expected_type,
            rhs_type,
        } => EvalError::InvalidType {
            expected_type,
            found_type: *rhs_type,
            found_span: rhs_span,
        },
        BinaryEvalError::ExponentHasUnits { exponent_unit } => EvalError::ExponentHasUnits {
            exponent_span: rhs_span,
            exponent_unit,
        },
        BinaryEvalError::ExponentIsInterval { exponent_interval } => {
            EvalError::ExponentIsInterval {
                exponent_interval,
                exponent_value_span: rhs_span,
            }
        }
    }
}

/// Converts a binary eval error that only applies to the left-hand side into an evaluator error.
///
/// # Panics
///
/// Panics if the error is not `InvalidLhsType`.
#[must_use]
pub fn binary_eval_error_expect_only_lhs(error: BinaryEvalError, lhs_span: Span) -> EvalError {
    match error {
        BinaryEvalError::InvalidLhsType {
            expected_type,
            lhs_type,
        } => EvalError::InvalidType {
            expected_type,
            found_type: *lhs_type,
            found_span: lhs_span,
        },
        BinaryEvalError::UnitMismatch { .. }
        | BinaryEvalError::TypeMismatch { .. }
        | BinaryEvalError::InvalidRhsType { .. }
        | BinaryEvalError::ExponentHasUnits { .. }
        | BinaryEvalError::ExponentIsInterval { .. } => {
            panic!("expected only lhs errors, but got {error:?}")
        }
    }
}

/// Converts a binary eval error that only applies to the right-hand side into an evaluator error.
///
/// # Panics
///
/// Panics if the error is not `InvalidRhsType`, `ExponentHasUnits`, or `ExponentIsInterval`.
#[must_use]
pub fn binary_eval_error_expect_only_rhs(error: BinaryEvalError, rhs_span: Span) -> EvalError {
    match error {
        BinaryEvalError::InvalidRhsType {
            expected_type,
            rhs_type,
        } => EvalError::InvalidType {
            expected_type,
            found_type: *rhs_type,
            found_span: rhs_span,
        },
        BinaryEvalError::ExponentHasUnits { exponent_unit } => EvalError::ExponentHasUnits {
            exponent_span: rhs_span,
            exponent_unit,
        },
        BinaryEvalError::ExponentIsInterval { exponent_interval } => {
            EvalError::ExponentIsInterval {
                exponent_interval,
                exponent_value_span: rhs_span,
            }
        }
        BinaryEvalError::UnitMismatch { .. }
        | BinaryEvalError::TypeMismatch { .. }
        | BinaryEvalError::InvalidLhsType { .. } => {
            panic!("expected only rhs errors, but got {error:?}")
        }
    }
}

/// Converts a unary eval error from the output crate into an evaluator error.
#[must_use]
pub fn unary_eval_error_to_eval_error(error: UnaryEvalError, value_span: Span) -> EvalError {
    match error {
        UnaryEvalError::InvalidNegType { value_type } => EvalError::InvalidType {
            expected_type: ExpectedType::Number { number_type: None },
            found_type: *value_type,
            found_span: value_span,
        },
        UnaryEvalError::InvalidNotType { value_type } => EvalError::InvalidType {
            expected_type: ExpectedType::Boolean,
            found_type: *value_type,
            found_span: value_span,
        },
    }
}
