//! Errors that can occur when evaluating binary or unary operations on values.
//!
//! These error types are used during expression evaluation. Conversion to
//! evaluator-level errors (`EvalError`) is done by the `oneil_eval` crate.

mod binary;
mod expected_type;
mod unary;
mod unit_conversion;

pub use binary::BinaryEvalError;
pub use expected_type::ExpectedType;
pub use unary::UnaryEvalError;
pub use unit_conversion::UnitConversionError;
