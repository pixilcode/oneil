//! Errors from resolving `design`, `use design`, and overlay declarations.

use std::fmt;

use oneil_shared::{
    error::{AsOneilError, ErrorLocation},
    span::Span,
};

/// A design-surface resolution failure (`design` / `use design` / shorthand rules).
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

    /// Returns the source span associated with this error.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for DesignResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl AsOneilError for DesignResolutionError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn error_location(&self, source: &str) -> Option<ErrorLocation> {
        Some(ErrorLocation::from_source_and_span(source, self.span))
    }
}
