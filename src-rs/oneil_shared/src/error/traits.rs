use crate::error::{Context, DiagnosticKind, ErrorLocation};

/// Trait for types that can be converted to Oneil diagnostics.
///
/// This trait provides a standardized interface for error types to expose
/// their diagnostic kind, message, and associated context. It is used throughout the
/// Oneil compiler and parser to ensure consistent diagnostic reporting.
pub trait AsOneilDiagnostic {
    /// Returns how this diagnostic should be classified (e.g. error vs. future severities).
    fn kind(&self) -> DiagnosticKind;

    /// Returns the primary diagnostic message.
    ///
    /// This should be a concise, user-friendly description of what went wrong.
    /// The message should be clear enough for users to understand the issue
    /// without requiring additional context.
    fn message(&self) -> String;

    /// Returns additional context information about the diagnostic.
    ///
    /// Context provides supplementary information that can help users understand
    /// the issue better or suggest how to fix it. This might include:
    /// - Notes with additional context
    /// - Help text with suggestions for fixing the issue
    /// - References to related code locations
    /// - Examples of correct usage
    ///
    /// Returns an empty vector if no context is available.
    fn context(&self) -> Vec<Context> {
        vec![]
    }

    /// Returns the location of the issue in the source code.
    ///
    /// This method should analyze the provided source code and return the
    /// precise location (line and column) where the issue occurred. The
    /// location information is used for highlighting in the source
    /// code during reporting.
    fn diagnostic_location(&self, source: &str) -> Option<ErrorLocation> {
        let _ = source;
        None
    }

    /// Returns context with optional source code locations.
    ///
    /// Similar to `context()`, but each context item can optionally include a specific
    /// location in the source code. This is useful when context refers to
    /// specific parts of the code (e.g., "variable 'x' was declared here").
    fn context_with_source(&self, source: &str) -> Vec<(Context, Option<ErrorLocation>)> {
        let _ = source;
        vec![]
    }

    /// Returns whether this diagnostic represents an internal diagnostic.
    ///
    /// Internal diagnostics are not important for the user to see,
    /// such as diagnostics that are caused by other errors. In that case, it's most
    /// useful to see only the first error.
    fn is_internal_diagnostic(&self) -> bool {
        false
    }
}
