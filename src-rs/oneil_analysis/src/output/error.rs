//! Error types for runtime output operations.

use indexmap::{IndexMap, IndexSet};
use oneil_shared::{
    paths::ModelPath,
    symbols::{ParameterName, TestIndex},
};

/// Singleton error indicating that model evaluation had errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelEvalHasErrors;

/// Accumulated errors encountered while building a dependency or reference tree.
#[derive(Debug)]
pub struct TreeErrors {
    errors: IndexMap<ModelPath, TreeModelError>,
}

impl TreeErrors {
    /// Creates an empty error collection.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            errors: IndexMap::new(),
        }
    }

    /// Records a model-level error for the given path.
    pub fn insert_model_error(&mut self, model_path: ModelPath) {
        self.errors.insert(model_path, TreeModelError::ModelError);
    }

    /// Records a parameter-level error for the given model path and parameter name.
    pub fn insert_parameter_error(&mut self, model_path: ModelPath, parameter_name: ParameterName) {
        self.errors
            .entry(model_path)
            .or_insert(TreeModelError::ModelError)
            .insert_parameter_error(parameter_name);
    }

    /// Records a test-level error for the given model path and test index.
    pub fn insert_test_error(&mut self, model_path: ModelPath, test_index: TestIndex) {
        self.errors
            .entry(model_path)
            .or_insert(TreeModelError::ModelError)
            .insert_test_error(test_index);
    }

    /// Merges another `TreeErrors` into this one, combining errors per model path.
    pub fn extend(&mut self, other: Self) {
        for (path, error) in other.errors {
            self.errors
                .entry(path)
                .or_insert(TreeModelError::ModelError)
                .extend(error);
        }
    }

    /// Returns an iterator over the model paths that have errors.
    pub fn model_paths(&self) -> impl Iterator<Item = &ModelPath> {
        self.errors.keys()
    }
}

/// Errors for a single model when building a tree.
#[derive(Debug)]
pub enum TreeModelError {
    /// The model could not be loaded or evaluated.
    ModelError,
    /// The model loaded but some parameters and/or tests had errors.
    LocalErrors {
        /// Names of parameters that had errors.
        parameters: IndexSet<ParameterName>,
        /// Indices of tests that had errors.
        tests: IndexSet<TestIndex>,
    },
}

impl TreeModelError {
    /// Adds a parameter error to this model error.
    pub fn insert_parameter_error(&mut self, parameter_name: ParameterName) {
        match self {
            Self::ModelError => (),
            Self::LocalErrors { parameters, .. } => {
                parameters.insert(parameter_name);
            }
        }
    }

    /// Adds a test error to this model error.
    pub fn insert_test_error(&mut self, test_index: TestIndex) {
        match self {
            Self::ModelError => (),
            Self::LocalErrors { tests, .. } => {
                tests.insert(test_index);
            }
        }
    }

    /// Merges another `TreeModelError` into this one.
    pub fn extend(&mut self, other: Self) {
        match (self, other) {
            (Self::ModelError, _) => (),
            (self_, other @ Self::ModelError) => *self_ = other,
            (
                Self::LocalErrors { parameters, tests },
                Self::LocalErrors {
                    parameters: other_parameters,
                    tests: other_tests,
                },
            ) => {
                parameters.extend(other_parameters);
                tests.extend(other_tests);
            }
        }
    }
}

/// Set of model paths that had errors during independents analysis.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndependentsErrors {
    /// Model paths that could not be evaluated or had evaluation errors.
    paths: IndexSet<ModelPath>,
}

impl IndependentsErrors {
    /// Creates an empty error set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: IndexSet::new(),
        }
    }

    /// Adds a model path that had an error.
    pub fn insert(&mut self, path: ModelPath) {
        self.paths.insert(path);
    }

    /// Returns whether no errors were recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Returns an iterator over the model paths that had errors.
    #[must_use]
    pub fn paths(&self) -> indexmap::set::Iter<'_, ModelPath> {
        self.paths.iter()
    }

    /// Adds all paths from another `IndependentsErrors` into this one.
    pub fn extend(&mut self, other: Self) {
        self.paths.extend(other.paths);
    }
}

/// Error when looking up a value for a tree node.
#[derive(Debug)]
pub enum GetValueError {
    /// The model was not found or could not be evaluated.
    Model,
    /// The parameter was not found in the model.
    Parameter,
}

/// Error when looking up a test value for a tree node.
#[derive(Debug)]
pub enum GetTestValueError {
    /// The model was not found or could not be evaluated.
    Model,
    /// The test was not found in the evaluated model.
    Test,
}
