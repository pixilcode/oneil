//! Data structures for evaluated model output.
//!
//! These types represent the results of evaluating Oneil models, including
//! parameters, tests, and submodels.

use indexmap::{IndexMap, IndexSet};

use oneil_shared::labels::ParameterLabel;
use oneil_shared::paths::ModelPath;
use oneil_shared::span::Span;
use oneil_shared::symbols::{BuiltinValueName, ParameterName, ReferenceName, TestIndex};
use oneil_shared::{EvalInstanceKey, InstancePath};

use crate::{EvalWarning, Value, dependency::DependencySet};

/// The result of evaluating a model.
///
/// This structure represents a fully evaluated model, containing all evaluated
/// parameters, tests, and recursively evaluated submodels. It is produced by
/// the evaluation process and can be used for output, further processing, or
/// analysis.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Model {
    /// The file path of the model that was evaluated.
    pub path: ModelPath,
    /// Import chain from the evaluation root to this model instance.
    pub instance_path: InstancePath,
    /// Aliases of submodel imports declared on this model.
    ///
    /// Each entry is a submodel's alias (= reference name), the same key used
    /// in [`Self::references`]. The set is provided so consumers can quickly
    /// distinguish references that originated as `use` submodels from plain
    /// `ref` references without re-walking the IR.
    pub submodels: IndexSet<ReferenceName>,
    /// A map of reference names to evaluated child instances.
    ///
    /// Each value identifies a distinct evaluation of a model file (path plus
    /// instance path), so the same file imported twice under different aliases
    /// can coexist in the evaluation cache.
    pub references: IndexMap<ReferenceName, EvalInstanceKey>,
    /// A map of parameter identifiers to their evaluated results.
    ///
    /// Parameters are stored by their identifier (name) and contain their
    /// evaluated values, units, and metadata.
    pub parameters: IndexMap<ParameterName, Parameter>,
    /// A list of evaluated test results.
    ///
    /// Tests are evaluated expressions that verify model behavior. Each test
    /// contains the evaluated value and the span of the original expression.
    pub tests: IndexMap<TestIndex, Test>,
}

/// The result of evaluating a test expression.
///
/// Tests are boolean expressions that verify expected behavior in a model.
/// This structure contains the evaluated value (which should be a boolean)
/// and the source location of the test expression.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Test {
    /// Source span of the test expression.
    pub expr_span: Span,
    /// The evaluated result of the test expression.
    pub result: TestResult,
    /// Warnings produced while evaluating the test expression (e.g. Python fallback).
    pub warnings: Vec<EvalWarning>,
}

impl Test {
    /// Returns whether the test passed.
    #[must_use]
    pub const fn passed(&self) -> bool {
        matches!(self.result, TestResult::Passed)
    }
}

/// The result of evaluating a test.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestResult {
    /// The test passed.
    Passed,
    /// The test failed.
    Failed {
        /// The values of the test dependencies.
        debug_info: Box<DebugInfo>,
    },
}

/// The result of evaluating a parameter.
///
/// Parameters are the primary data elements in a model. This structure
/// contains the evaluated value, associated unit (if any), and metadata about
/// the parameter such as whether it's a performance parameter and its
/// dependencies.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Parameter {
    /// The identifier (name) of the parameter.
    pub ident: ParameterName,
    /// The human-readable label for the parameter.
    pub label: ParameterLabel,
    /// The evaluated value of the parameter.
    pub value: Value,
    /// The print level for this parameter.
    ///
    /// This determines the level of debugging/tracing information that should
    /// be generated for this parameter during output.
    pub print_level: PrintLevel,
    /// The debug information for this parameter, if requested.
    pub debug_info: Option<DebugInfo>,
    /// The dependencies of this parameter.
    pub dependencies: DependencySet,
    /// The span of the parameter expression.
    pub expr_span: Span,
    /// Warnings produced while evaluating this parameter (e.g. Python fallback).
    pub warnings: Vec<EvalWarning>,
}

impl Parameter {
    /// Returns whether this parameter should be printed at
    /// the given print level.
    #[must_use]
    pub fn should_print(&self, print_level: PrintLevel) -> bool {
        self.print_level >= print_level
    }
}

/// Debug information for a parameter.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DebugInfo {
    /// The values of the builtin dependencies at the time the parameter was evaluated.
    pub builtin_dependency_values: IndexMap<BuiltinValueName, Value>,
    /// The values of the parameter dependencies at the time the parameter was evaluated.
    pub parameter_dependency_values: IndexMap<ParameterName, Value>,
    /// The values of the external dependencies at the time the parameter was evaluated.
    pub external_dependency_values: IndexMap<(ReferenceName, ParameterName), Value>,
}

/// The trace level for debugging and diagnostic output.
///
/// Trace levels control the verbosity of debugging information during model
/// evaluation. Higher levels provide more detailed information.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PrintLevel {
    /// No output.
    None,
    /// Basic tracing output.
    Trace,
    /// Performance output.
    Performance,
}
