//! Circular dependency errors for the Oneil model loader.

use std::fmt;

use oneil_shared::{
    error::{AsOneilDiagnostic, DiagnosticKind},
    paths::ModelPath,
};

/// Represents a circular dependency detected during model loading.
///
/// A circular dependency occurs when model A depends on model B, which depends on
/// model C, which depends back on model A (or any other cycle). This error contains
/// the complete cycle of model paths that form the circular dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularDependencyError(Vec<ModelPath>);

impl CircularDependencyError {
    /// Creates a new circular dependency error.
    #[must_use]
    pub const fn new(circular_dependency: Vec<ModelPath>) -> Self {
        Self(circular_dependency)
    }

    /// Returns the circular dependency path.
    #[must_use]
    pub const fn circular_dependency(&self) -> &[ModelPath] {
        self.0.as_slice()
    }
}

impl fmt::Display for CircularDependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let circular_dependency_str = self
            .0
            .iter()
            .map(|path| path.as_path().display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        write!(
            f,
            "circular dependency found in models - {circular_dependency_str}"
        )
    }
}

impl AsOneilDiagnostic for CircularDependencyError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }
}
