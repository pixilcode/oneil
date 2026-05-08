//! Errors from resolving `design`, `apply`, and overlay declarations.

use std::fmt;

use oneil_shared::{
    error::{AsOneilDiagnostic, DiagnosticKind, ErrorLocation},
    span::Span,
};

/// A design-surface resolution failure (`design` / `apply` / shorthand rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignResolutionError {
    message: String,
    span: Span,
}

impl DesignResolutionError {
    /// Creates an error with the user-facing message and the primary source span.
    #[must_use]
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Returns the user-facing error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the source span associated with this error.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span.clone()
    }
}

impl fmt::Display for DesignResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl AsOneilDiagnostic for DesignResolutionError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn diagnostic_location(&self, _source: &str) -> Option<ErrorLocation> {
        Some(ErrorLocation::from_span(&self.span))
    }
}
