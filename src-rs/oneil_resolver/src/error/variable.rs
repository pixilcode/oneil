use std::fmt;

use oneil_shared::{
    error::{AsOneilError, Context, ErrorLocation},
    paths::{ModelPath, PythonPath},
    span::Span,
    symbols::{ParameterName, ReferenceName},
};

use super::unit::UnitResolutionError;

/// Represents an error that occurred during variable resolution within expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableResolutionError {
    /// The model that should contain the variable has errors.
    ModelHasError {
        /// The path of the model that has errors
        path: ModelPath,
        /// The span of where the model is referenced
        reference_span: Span,
    },
    /// The parameter that should contain the variable has errors.
    ParameterHasError {
        /// The identifier of the parameter that has errors
        parameter_name: ParameterName,
        /// The span of where the parameter is referenced
        reference_span: Span,
    },
    /// The resolution of a submodel that is referenced by a variable has failed.
    ReferenceResolutionFailed {
        /// The identifier of the reference that has errors
        identifier: ReferenceName,
        /// The span of where the reference is referenced
        reference_span: Span,
    },
    /// The parameter is not defined in the current context.
    UndefinedParameter {
        /// The path of the model that contains the parameter (if None, the parameter is not defined in the current model)
        model_path: Option<ModelPath>,
        /// The identifier of the parameter that is undefined
        parameter_name: ParameterName,
        /// The span of where the parameter is referenced
        reference_span: Span,
        /// Best match for the parameter name
        best_match: Option<String>,
    },
    /// The reference is not defined in the current model.
    UndefinedReference {
        /// The identifier of the reference that is undefined
        reference: ReferenceName,
        /// The span of where the reference is referenced
        reference_span: Span,
        /// Best match for the reference name
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
    /// Creates a new error indicating that the model has errors.
    #[must_use]
    pub const fn model_has_error(model_path: ModelPath, reference_span: Span) -> Self {
        Self::ModelHasError {
            path: model_path,
            reference_span,
        }
    }

    /// Creates a new error indicating that the parameter has errors.
    #[must_use]
    pub const fn parameter_has_error(parameter_name: ParameterName, reference_span: Span) -> Self {
        Self::ParameterHasError {
            parameter_name,
            reference_span,
        }
    }

    /// Creates a new error indicating that resolution of a submodel that is
    /// referenced by a variable has failed.
    #[must_use]
    pub const fn reference_resolution_failed(
        identifier: ReferenceName,
        reference_span: Span,
    ) -> Self {
        Self::ReferenceResolutionFailed {
            identifier,
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
            model_path: None,
            parameter_name,
            reference_span,
            best_match,
        }
    }

    /// Creates a new error indicating that the parameter is undefined in a specific reference.
    #[must_use]
    pub const fn undefined_parameter_in_reference(
        reference_path: ModelPath,
        parameter_name: ParameterName,
        reference_span: Span,
        best_match: Option<String>,
    ) -> Self {
        Self::UndefinedParameter {
            model_path: Some(reference_path),
            parameter_name,
            reference_span,
            best_match,
        }
    }

    /// Creates a new error indicating that the submodel is undefined in the current model.
    #[must_use]
    pub const fn undefined_reference(
        reference: ReferenceName,
        reference_span: Span,
        best_match: Option<String>,
    ) -> Self {
        Self::UndefinedReference {
            reference,
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
            Self::ModelHasError {
                path,
                reference_span: _,
            } => {
                let path = path.as_path().display();
                write!(f, "model `{path}` has errors")
            }
            Self::ParameterHasError {
                parameter_name,
                reference_span: _,
            } => {
                let identifier = parameter_name.as_str();
                write!(f, "parameter `{identifier}` has errors")
            }
            Self::ReferenceResolutionFailed {
                identifier,
                reference_span: _,
            } => {
                let identifier = identifier.as_str();
                write!(f, "unable to resolve submodel `{identifier}`")
            }
            Self::UndefinedParameter {
                model_path,
                parameter_name,
                reference_span: _,
                best_match: _,
            } => {
                let identifier_str = parameter_name.as_str();
                match model_path {
                    Some(path) => {
                        let path = path.as_path().display();
                        write!(
                            f,
                            "parameter `{identifier_str}` is not defined in model `{path}`"
                        )
                    }
                    None => write!(
                        f,
                        "parameter `{identifier_str}` is not defined in the current model"
                    ),
                }
            }
            Self::UndefinedReference {
                reference,
                reference_span: _,
                best_match: _,
            } => {
                let identifier_str = reference.as_str();
                write!(
                    f,
                    "reference `{identifier_str}` is not defined in the current model"
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

impl AsOneilError for VariableResolutionError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn error_location(&self, source: &str) -> Option<ErrorLocation> {
        match self {
            Self::ModelHasError {
                path: _,
                reference_span,
            }
            | Self::ParameterHasError {
                parameter_name: _,
                reference_span,
            }
            | Self::ReferenceResolutionFailed {
                identifier: _,
                reference_span,
            }
            | Self::UndefinedParameter {
                model_path: _,
                parameter_name: _,
                reference_span,
                best_match: _,
            }
            | Self::UndefinedReference {
                reference: _,
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
            Self::UnitResolution(unit_error) => unit_error.error_location(source),
        }
    }

    fn context(&self) -> Vec<Context> {
        match self {
            Self::ModelHasError {
                path: _,
                reference_span: _,
            }
            | Self::ParameterHasError {
                parameter_name: _,
                reference_span: _,
            }
            | Self::ReferenceResolutionFailed {
                identifier: _,
                reference_span: _,
            }
            | Self::UnitResolution(_) => Vec::new(),
            Self::UndefinedParameter {
                model_path: _,
                parameter_name: _,
                reference_span: _,
                best_match,
            }
            | Self::UndefinedReference {
                reference: _,
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

    fn is_internal_error(&self) -> bool {
        matches!(
            self,
            Self::ModelHasError { .. }
                | Self::ParameterHasError { .. }
                | Self::ReferenceResolutionFailed { .. }
        )
    }
}

impl From<UnitResolutionError> for VariableResolutionError {
    fn from(error: UnitResolutionError) -> Self {
        Self::UnitResolution(error)
    }
}
