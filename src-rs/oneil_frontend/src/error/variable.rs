use std::fmt;

use oneil_shared::{
    error::{AsOneilDiagnostic, Context, DiagnosticKind, ErrorLocation},
    paths::PythonPath,
    span::Span,
    symbols::ParameterName,
};

use super::unit::UnitResolutionError;

/// Represents an error that occurred during variable resolution within expressions.
///
/// Note: undefined references and undefined reference parameters are **not** surfaced
/// here; they are deferred to the post-build validation pass so that designs applied
/// at instance time can contribute references and parameters not visible at file time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableResolutionError {
    /// The parameter that should contain the variable has errors.
    ParameterHasError {
        /// The identifier of the parameter that has errors
        parameter_name: ParameterName,
        /// The span of where the parameter is referenced
        reference_span: Span,
    },
    /// The parameter is not defined in the current model scope.
    UndefinedParameter {
        /// The identifier of the parameter that is undefined
        parameter_name: ParameterName,
        /// The span of where the parameter is referenced
        reference_span: Span,
        /// Best match for the parameter name
        best_match: Option<String>,
    },
    /// The function name is not defined as a builtin or in any loaded Python import.
    UndefinedFunction {
        /// The function name that could not be resolved
        function_name: String,
        /// The span of the function name in the source
        relevant_span: Span,
        /// Best match for the function name
        best_match: Option<String>,
    },
    /// The function name is defined in more than one Python import; resolution is ambiguous.
    MultipleFunctionsFound {
        /// The function name that was resolved in multiple imports
        function_name: String,
        /// The span of the function name in the source
        relevant_span: Span,
        /// The Python paths that export this function
        python_paths: Vec<PythonPath>,
    },
    /// A unit resolution error occurred (e.g. in a unit cast expression).
    UnitResolution(UnitResolutionError),
}

impl VariableResolutionError {
    /// Creates a new error indicating that the parameter has errors.
    #[must_use]
    pub const fn parameter_has_error(parameter_name: ParameterName, reference_span: Span) -> Self {
        Self::ParameterHasError {
            parameter_name,
            reference_span,
        }
    }

    /// Creates a new error indicating that the parameter is undefined in the current model.
    #[must_use]
    pub const fn undefined_parameter(
        parameter_name: ParameterName,
        reference_span: Span,
        best_match: Option<String>,
    ) -> Self {
        Self::UndefinedParameter {
            parameter_name,
            reference_span,
            best_match,
        }
    }

    /// Creates a new error indicating that the function is not defined as a builtin or in any Python import.
    #[must_use]
    pub const fn undefined_function(
        function_name: String,
        relevant_span: Span,
        best_match: Option<String>,
    ) -> Self {
        Self::UndefinedFunction {
            function_name,
            relevant_span,
            best_match,
        }
    }

    /// Creates a new error indicating that the function is defined in more than one Python import.
    #[must_use]
    pub const fn multiple_functions_found(
        function_name: String,
        relevant_span: Span,
        python_paths: Vec<PythonPath>,
    ) -> Self {
        Self::MultipleFunctionsFound {
            function_name,
            relevant_span,
            python_paths,
        }
    }
}

impl fmt::Display for VariableResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParameterHasError {
                parameter_name,
                reference_span: _,
            } => {
                let identifier = parameter_name.as_str();
                write!(f, "parameter `{identifier}` has errors")
            }
            Self::UndefinedParameter {
                parameter_name,
                reference_span: _,
                best_match: _,
            } => {
                let identifier_str = parameter_name.as_str();
                write!(
                    f,
                    "parameter `{identifier_str}` is not defined in the current model"
                )
            }
            Self::UndefinedFunction {
                function_name,
                relevant_span: _,
                best_match: _,
            } => {
                write!(
                    f,
                    "function `{function_name}` is not defined as a builtin or in any Python import"
                )
            }
            Self::MultipleFunctionsFound {
                function_name,
                python_paths: _,
                relevant_span: _,
            } => {
                write!(
                    f,
                    "function `{function_name}` is defined in multiple Python imports"
                )
            }
            Self::UnitResolution(unit_error) => unit_error.fmt(f),
        }
    }
}

impl AsOneilDiagnostic for VariableResolutionError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn diagnostic_location(&self, source: &str) -> Option<ErrorLocation> {
        match self {
            Self::ParameterHasError {
                parameter_name: _,
                reference_span,
            }
            | Self::UndefinedParameter {
                parameter_name: _,
                reference_span,
                best_match: _,
            } => {
                let location = ErrorLocation::from_source_and_span(source, *reference_span);
                Some(location)
            }
            Self::UndefinedFunction {
                function_name: _,
                relevant_span,
                best_match: _,
            }
            | Self::MultipleFunctionsFound {
                function_name: _,
                relevant_span,
                python_paths: _,
            } => {
                let location = ErrorLocation::from_source_and_span(source, *relevant_span);
                Some(location)
            }
            Self::UnitResolution(unit_error) => unit_error.diagnostic_location(source),
        }
    }

    fn context(&self) -> Vec<Context> {
        match self {
            Self::ParameterHasError {
                parameter_name: _,
                reference_span: _,
            }
            | Self::UnitResolution(_) => Vec::new(),
            Self::UndefinedParameter {
                parameter_name: _,
                reference_span: _,
                best_match,
            }
            | Self::UndefinedFunction {
                function_name: _,
                relevant_span: _,
                best_match,
            } => best_match.as_ref().map_or_else(Vec::new, |best_match| {
                vec![Context::Help(format!("did you mean `{best_match}`?"))]
            }),
            Self::MultipleFunctionsFound {
                function_name,
                relevant_span: _,
                python_paths,
            } => python_paths
                .iter()
                .map(|path| {
                    Context::Note(format!(
                        "python import `{}` exports `{function_name}`",
                        path.as_path().display()
                    ))
                })
                .collect(),
        }
    }

    fn is_internal_diagnostic(&self) -> bool {
        matches!(self, Self::ParameterHasError { .. })
    }
}

impl From<UnitResolutionError> for VariableResolutionError {
    fn from(error: UnitResolutionError) -> Self {
        Self::UnitResolution(error)
    }
}
