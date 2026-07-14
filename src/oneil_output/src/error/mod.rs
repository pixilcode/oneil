//! Errors for Oneil: value-level checked operations, evaluation-time diagnostics, and conversions
//! between them.

mod binary;
pub mod convert;
mod eval_error;
mod eval_warning;
mod expected_argument_count;
mod expected_type;
mod model_eval_errors;
mod unary;
mod unit_conversion;

pub use binary::BinaryEvalError;
pub use eval_error::EvalError;
pub use eval_warning::EvalWarning;
pub use expected_argument_count::ExpectedArgumentCount;
pub use expected_type::ExpectedType;
pub use model_eval_errors::ModelEvalErrors;
pub use unary::UnaryEvalError;
pub use unit_conversion::UnitConversionError;
