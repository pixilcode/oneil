use std::fmt;

use oneil_shared::{
    error::{AsOneilDiagnostic, Context, DiagnosticKind, ErrorLocation},
    span::Span,
    symbols::ParameterName,
};

use super::unit::UnitResolutionError;
use super::variable::VariableResolutionError;

/// Represents an error that occurred during parameter resolution.
///
/// Cycles in the parameter dependency graph are no longer surfaced from
/// here: the post-build SCC pass in
/// [`oneil_analysis::validate_instance_graph`] runs over the composed
/// graph (post-build, post-all-applies) and emits
/// `InstanceValidationErrorKind::ParameterCycle` per cycle member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterResolutionError {
    /// A variable resolution error occurred within the parameter's value.
    VariableResolution(VariableResolutionError),
    /// A unit resolution error occurred within the parameter's unit.
    UnitResolution(UnitResolutionError),
    /// A duplicate parameter was detected.
    DuplicateParameter {
        /// The identifier of the parameter.
        parameter_name: ParameterName,
        /// The span of the original parameter.
        original_span: Span,
        /// The span of the duplicate parameter.
        duplicate_span: Span,
    },
}

impl ParameterResolutionError {
    /// Creates a new error indicating a variable resolution error within a parameter.
    #[must_use]
    pub const fn variable_resolution(error: VariableResolutionError) -> Self {
        Self::VariableResolution(error)
    }

    /// Creates a new error indicating a unit resolution error within a parameter.
    #[must_use]
    pub const fn unit_resolution(error: UnitResolutionError) -> Self {
        Self::UnitResolution(error)
    }

    /// Creates a new error indicating a duplicate parameter was detected.
    #[must_use]
    pub const fn duplicate_parameter(
        parameter_name: ParameterName,
        original_span: Span,
        duplicate_span: Span,
    ) -> Self {
        Self::DuplicateParameter {
            parameter_name,
            original_span,
            duplicate_span,
        }
    }
}

impl fmt::Display for ParameterResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VariableResolution(variable_error) => variable_error.fmt(f),
            Self::UnitResolution(unit_error) => unit_error.fmt(f),
            Self::DuplicateParameter { parameter_name, .. } => {
                write!(f, "duplicate parameter `{}`", parameter_name.as_str())
            }
        }
    }
}

impl From<VariableResolutionError> for ParameterResolutionError {
    fn from(error: VariableResolutionError) -> Self {
        Self::variable_resolution(error)
    }
}

impl From<UnitResolutionError> for ParameterResolutionError {
    fn from(error: UnitResolutionError) -> Self {
        Self::unit_resolution(error)
    }
}

impl AsOneilDiagnostic for ParameterResolutionError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn diagnostic_location(&self, source: &str) -> Option<ErrorLocation> {
        match self {
            Self::VariableResolution(error) => error.diagnostic_location(source),
            Self::UnitResolution(error) => error.diagnostic_location(source),
            Self::DuplicateParameter { duplicate_span, .. } => {
                let location = ErrorLocation::from_span(duplicate_span);
                Some(location)
            }
        }
    }

    fn context_with_source(&self, source: &str) -> Vec<(Context, Option<ErrorLocation>)> {
        match self {
            Self::VariableResolution(error) => error.context_with_source(source),
            Self::UnitResolution(error) => error.context_with_source(source),
            Self::DuplicateParameter { original_span, .. } => {
                let original_location = ErrorLocation::from_span(original_span);
                let context = Context::Note("original parameter found here".to_string());
                vec![(context, Some(original_location))]
            }
        }
    }

    fn is_internal_diagnostic(&self) -> bool {
        match self {
            Self::VariableResolution(error) => error.is_internal_diagnostic(),
            Self::UnitResolution(error) => error.is_internal_diagnostic(),
            Self::DuplicateParameter { .. } => false,
        }
    }
}
