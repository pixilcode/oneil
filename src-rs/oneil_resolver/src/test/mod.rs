//! Helper functions for creating test data
//!
//! Creating test data can be a tedious and repetitive process, especially where `Span`s are
//! involved. This module provides helper functions to create test data that can be used in tests.

use std::path::PathBuf;

use oneil_shared::{paths::ModelPath, span::Span};

pub mod external_context;
pub mod resolution_context;
pub mod test_ast;
pub mod test_ir;

pub fn unimportant_span() -> Span {
    Span::random_span()
}

pub fn test_model_path(s: &str) -> ModelPath {
    let path = PathBuf::from(s);
    ModelPath::from_path_no_ext(&path)
}

/// Returns the path for a sibling model relative to `model_path`.
///
/// The sibling name must not include the `.on` extension; it is added automatically.
#[must_use]
pub fn test_model_sibling_path(model_path: &ModelPath, sibling_name: &str) -> ModelPath {
    let sibling = ModelPath::from_str_no_ext(sibling_name);
    model_path.get_sibling_model_path(sibling)
}
