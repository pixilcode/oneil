use indexmap::{IndexMap, IndexSet};
use oneil_shared::{
    paths::ModelPath,
    symbols::{ParameterName, TestIndex},
};

use super::EvalError;

/// Errors collected while evaluating a single model (parameters, tests, and broken references).
#[derive(Debug, Clone)]
pub struct ModelEvalErrors {
    /// Errors that occurred during evaluation of the parameters.
    pub parameters: IndexMap<ParameterName, Vec<EvalError>>,
    /// Errors that occurred during evaluation of the tests.
    pub tests: IndexMap<TestIndex, Vec<EvalError>>,
    /// References that had errors.
    pub references: IndexSet<ModelPath>,
}
