use std::{error::Error, fmt};

use oneil_ast as ast;
use oneil_shared::{
    error::{AsOneilDiagnostic, DiagnosticKind, ErrorLocation},
    span::Span,
};

/// Represents an error that occurred during unit resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitResolutionError {
    /// The full unit name that could not be resolved
    unit_name: ast::UnitIdentifier,
    /// The span of the unit name in the source
    unit_name_span: Span,
}

impl UnitResolutionError {
    /// Creates a new error indicating that a unit could not be resolved.
    #[must_use]
    pub const fn new(unit_name: ast::UnitIdentifier, unit_name_span: Span) -> Self {
        Self {
            unit_name,
            unit_name_span,
        }
    }

    /// Returns the name of the unit that could not be resolved.
    #[must_use]
    pub const fn unit_name(&self) -> &ast::UnitIdentifier {
        &self.unit_name
    }

    /// Returns the span of the unit name in the source.
    #[must_use]
    pub fn unit_name_span(&self) -> Span {
        self.unit_name_span.clone()
    }
}

impl fmt::Display for UnitResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown unit `{}`", self.unit_name.as_str())
    }
}

impl Error for UnitResolutionError {}

impl AsOneilDiagnostic for UnitResolutionError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn diagnostic_location(&self, _source: &str) -> Option<ErrorLocation> {
        let location = ErrorLocation::from_span(&self.unit_name_span);
        Some(location)
    }
}
