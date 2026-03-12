use std::fmt;

use oneil_shared::{
    error::{AsOneilError, Context, ErrorLocation},
    paths::ModelPath,
    span::Span,
    symbols::{ReferenceName, SubmodelName},
};

/// Represents an error that occurred during submodel resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelImportResolutionError {
    /// The referenced model has errors, preventing submodel resolution.
    ModelHasError {
        /// The path of the model that has errors
        model_path: ModelPath,
        /// The span of where the model is referenced
        reference_span: Span,
    },
    /// Parent model has errors, preventing submodel resolution.
    ParentModelHasError {
        /// The identifier of the parent model that has errors
        parent_model_name: SubmodelName,
        /// The span of where the parent model is referenced
        parent_model_name_span: Span,
        /// The identifier of the submodel that is undefined
        submodel_name: ReferenceName,
        /// The span of where the submodel is referenced
        submodel_name_span: Span,
    },
    /// The submodel identifier is not defined in the referenced model.
    UndefinedSubmodel {
        /// The path of the model that contains the submodel
        parent_model_path: ModelPath,
        /// The identifier of the submodel that is undefined
        submodel: SubmodelName,
        /// The span of where the submodel is referenced
        reference_span: Span,
    },
    /// The submodel name is a duplicate.
    DuplicateSubmodel {
        /// The identifier of the duplicate submodel
        submodel: SubmodelName,
        /// The span of where the original submodel is referenced
        original_span: Span,
        /// The span of where the duplicate submodel is referenced
        duplicate_span: Span,
    },
    /// The reference name is a duplicate.
    DuplicateReference {
        /// The identifier of the duplicate reference
        reference: ReferenceName,
        /// The span of where the original reference is referenced
        original_span: Span,
        /// The span of where the duplicate reference is referenced
        duplicate_span: Span,
    },
}

impl ModelImportResolutionError {
    /// Creates a new error indicating that the referenced model has errors.
    #[must_use]
    pub const fn model_has_error(model_path: ModelPath, reference_span: Span) -> Self {
        Self::ModelHasError {
            model_path,
            reference_span,
        }
    }

    /// Creates a new error indicating that the parent model has errors.
    #[must_use]
    pub const fn parent_model_has_error(
        parent_model_name: SubmodelName,
        parent_model_name_span: Span,
        submodel_name: ReferenceName,
        submodel_name_span: Span,
    ) -> Self {
        Self::ParentModelHasError {
            parent_model_name,
            parent_model_name_span,
            submodel_name,
            submodel_name_span,
        }
    }

    /// Creates a new error indicating that the submodel is undefined in the referenced model.
    #[must_use]
    pub const fn undefined_submodel_in_submodel(
        parent_model_path: ModelPath,
        submodel: SubmodelName,
        reference_span: Span,
    ) -> Self {
        Self::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span,
        }
    }

    /// Creates a new error indicating that the submodel name is a duplicate.
    #[must_use]
    pub const fn duplicate_submodel(
        submodel: SubmodelName,
        original_span: Span,
        duplicate_span: Span,
    ) -> Self {
        Self::DuplicateSubmodel {
            submodel,
            original_span,
            duplicate_span,
        }
    }

    /// Creates a new error indicating that the reference name is a duplicate.
    #[must_use]
    pub const fn duplicate_reference(
        reference: ReferenceName,
        original_span: Span,
        duplicate_span: Span,
    ) -> Self {
        Self::DuplicateReference {
            reference,
            original_span,
            duplicate_span,
        }
    }
}

impl fmt::Display for ModelImportResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelHasError {
                model_path: path,
                reference_span: _,
            } => {
                let path = path.as_path().display();
                write!(f, "submodel `{path}` has errors")
            }
            Self::ParentModelHasError {
                parent_model_name,
                submodel_name,
                ..
            } => {
                let submodel_name = submodel_name.as_str();
                let parent_model_name = parent_model_name.as_str();
                write!(
                    f,
                    "unable to resolve submodel `{submodel_name}` because parent model `{parent_model_name}` has errors"
                )
            }
            Self::UndefinedSubmodel {
                parent_model_path,
                submodel,
                reference_span: _,
            } => {
                let path = parent_model_path.as_path().display();
                let submodel_str = submodel.as_str();
                write!(
                    f,
                    "submodel `{submodel_str}` is not defined in model `{path}`"
                )
            }
            Self::DuplicateSubmodel { submodel, .. } => {
                let submodel_str = submodel.as_str();
                write!(f, "submodel `{submodel_str}` is defined multiple times")
            }
            Self::DuplicateReference { reference, .. } => {
                let reference_str = reference.as_str();
                write!(f, "reference `{reference_str}` is defined multiple times")
            }
        }
    }
}

impl AsOneilError for ModelImportResolutionError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn error_location(&self, source: &str) -> Option<ErrorLocation> {
        match self {
            Self::ModelHasError {
                model_path: _,
                reference_span: location_span,
            }
            | Self::ParentModelHasError {
                submodel_name_span: location_span,
                ..
            }
            | Self::UndefinedSubmodel {
                parent_model_path: _,
                submodel: _,
                reference_span: location_span,
            }
            | Self::DuplicateSubmodel {
                duplicate_span: location_span,
                ..
            }
            | Self::DuplicateReference {
                duplicate_span: location_span,
                ..
            } => {
                let location = ErrorLocation::from_source_and_span(source, *location_span);
                Some(location)
            }
        }
    }

    fn context_with_source(&self, source: &str) -> Vec<(Context, Option<ErrorLocation>)> {
        match self {
            Self::ParentModelHasError {
                parent_model_name,
                parent_model_name_span,
                ..
            } => {
                let model_name = parent_model_name.as_str();
                let location = ErrorLocation::from_source_and_span(source, *parent_model_name_span);
                vec![(
                    Context::Note(format!("model `{model_name}` failed to resolve")),
                    Some(location),
                )]
            }
            Self::DuplicateSubmodel { original_span, .. } => {
                let location = ErrorLocation::from_source_and_span(source, *original_span);
                vec![(
                    Context::Note("submodel is originally defined here".to_string()),
                    Some(location),
                )]
            }
            Self::DuplicateReference { original_span, .. } => {
                let location = ErrorLocation::from_source_and_span(source, *original_span);
                vec![(
                    Context::Note("reference is originally defined here".to_string()),
                    Some(location),
                )]
            }
            Self::ModelHasError { .. } | Self::UndefinedSubmodel { .. } => vec![],
        }
    }

    fn is_internal_error(&self) -> bool {
        matches!(
            self,
            Self::ModelHasError { .. } | Self::ParentModelHasError { .. }
        )
    }
}
