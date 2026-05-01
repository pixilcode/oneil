//! Error handling for the Oneil model loader.
//!
//! This module defines error types that can occur during the resolution phase of
//! model loading. Resolution errors occur when references cannot be resolved to
//! their actual definitions, such as when a submodel reference points to a
//! non-existent model or when a parameter reference cannot be found.
//!
//! # Error Categories
//!
//! - **Import errors**: Errors that occur during Python import validation
//! - **Submodel resolution errors**: Errors that occur when resolving `submodel`/`reference` declarations
//! - **Parameter resolution errors**: Errors that occur when resolving parameter references
//! - **Test resolution errors**: Errors that occur when resolving test references
//! - **Variable resolution errors**: Errors that occur when resolving variable references within expressions
//!
//! Cross-file cycles are detected at instance-graph build time rather
//! than during resolution; see
//! [`CompilationCycleError`](crate::CompilationCycleError) (in
//! [`crate::instance`]) for that diagnostic.

mod design;
mod errors;
mod model_import;
mod parameter;
mod python_import;
mod unit;
mod util;
mod variable;

pub use design::DesignResolutionError;
pub use errors::ResolutionErrorCollection;
pub use model_import::ModelImportResolutionError;
pub use parameter::ParameterResolutionError;
pub use python_import::PythonImportResolutionError;
pub use unit::UnitResolutionError;
pub use util::{combine_error_list, combine_errors, convert_errors, split_ok_and_errors};
pub use variable::VariableResolutionError;
