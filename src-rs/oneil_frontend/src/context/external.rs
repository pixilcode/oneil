use indexmap::IndexSet;
use oneil_ast as ast;
use oneil_output::Unit;
use oneil_shared::{
    load_result::LoadResult,
    paths::{ModelPath, PythonPath},
    symbols::{BuiltinFunctionName, BuiltinValueName, PyFunctionName, UnitBaseName, UnitPrefix},
};

use crate::{ResolutionErrorCollection, instance::InstancedModel};

/// Error indicating that loading/parsing a model's AST failed.
pub struct AstLoadingFailedError;

/// Error indicating that loading a Python import failed.
pub struct PythonImportLoadingFailedError;

/// Context provided by the environment for resolving models (builtins, AST loading, Python imports).
pub trait ExternalResolutionContext {
    /// Checks if the given identifier refers to a builtin value.
    fn has_builtin_value(&self, identifier: &ast::Identifier) -> bool;

    /// Checks if the given identifier refers to a builtin function.
    fn has_builtin_function(&self, identifier: &ast::Identifier) -> bool;

    /// Yields every builtin value name (for fuzzy matching and suggestions).
    fn get_builtin_value_names(&self) -> impl Iterator<Item = &BuiltinValueName>;

    /// Yields every builtin function name (for fuzzy matching and suggestions).
    fn get_builtin_function_names(&self) -> impl Iterator<Item = &BuiltinFunctionName>;

    /// Checks if the given name refers to a builtin unit.
    fn has_builtin_unit(&self, name: &str) -> bool;

    /// Returns the available unit prefixes (e.g., "k" -> 1000.0).
    fn available_prefixes(&self) -> impl Iterator<Item = (&UnitPrefix, f64)>;

    /// Returns whether the given unit name supports SI prefixes.
    fn unit_supports_si_prefixes(&self, name: &UnitBaseName) -> bool;

    /// Returns the resolved [`Unit`] for the given builtin base name, if any.
    ///
    /// Used during lowering to pre-compute each [`ir::CompositeUnit`]'s
    /// dimension map, so later passes don't need a runtime unit dictionary.
    fn lookup_unit(&self, name: &UnitBaseName) -> Option<&Unit>;

    /// Loads the AST for a model.
    ///
    /// # Errors
    ///
    /// Returns `Err(AstLoadingFailedError)` when the model file cannot be read or parsed.
    fn load_ast(&mut self, path: &ModelPath) -> LoadResult<&ast::ModelNode, AstLoadingFailedError>;

    /// Loads a Python import.
    ///
    /// # Errors
    ///
    /// Returns `Err(PythonImportLoadingFailedError)` when the Python import cannot be loaded or
    /// validated.
    fn load_python_import<'context>(
        &'context mut self,
        python_path: &PythonPath,
    ) -> Result<IndexSet<&'context PyFunctionName>, PythonImportLoadingFailedError>;

    /// Returns the pre-loaded model templates.
    ///
    /// Each item is the model path, a clone of its lowered template, and a
    /// clone of any resolver-time errors recorded for it. Owned values let
    /// implementors source this data from any backing store (e.g. the
    /// `unit_graph_cache`) without needing to return borrows into it.
    fn get_preloaded_models(
        &self,
    ) -> impl Iterator<Item = (ModelPath, InstancedModel, ResolutionErrorCollection)>;
}
