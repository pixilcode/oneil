//! IR loading and resolution for the runtime.

use indexmap::IndexSet;
use oneil_ir as ir;
use oneil_resolver::{
    self as resolver,
    error::{UnitResolutionError, VariableResolutionError},
};
use oneil_shared::{
    load_result::LoadResult,
    paths::{ModelPath, PythonPath},
    symbols::{BuiltinFunctionName, BuiltinValueName, PyFunctionName, UnitBaseName, UnitPrefix},
};

use super::Runtime;
use crate::output::{self, ast, error::RuntimeErrors};

impl Runtime {
    /// Loads the IR for a model and all of its dependencies.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeErrors`] (via [`get_model_errors`](super::Runtime::get_model_errors)) if that
    /// model had parse or resolution errors.
    pub fn load_ir(
        &mut self,
        path: &ModelPath,
    ) -> (
        Option<output::reference::ModelIrReference<'_>>,
        RuntimeErrors,
    ) {
        self.load_ir_internal(path);

        let ir_opt = self
            .ir_cache
            .get_entry(path)
            .and_then(LoadResult::value)
            .map(|ir| output::reference::ModelIrReference::new(ir, &self.ir_cache));

        let include_indirect_errors = true;

        let errors = self.get_model_errors(path, include_indirect_errors);

        (ir_opt, errors)
    }

    pub(super) fn load_ir_internal(
        &mut self,
        path: &ModelPath,
    ) -> &LoadResult<output::ir::Model, resolver::ResolutionErrorCollection> {
        let results = resolver::load_model(path, self);

        for (model_path, result) in results {
            let (model, model_errors) = result.into_parts();

            if model_errors.is_empty() {
                self.ir_cache.insert(model_path, LoadResult::success(model));
            } else {
                self.ir_cache
                    .insert(model_path, LoadResult::partial(model, model_errors));
            }
        }

        self.ir_cache
            .get_entry(path)
            .expect("entry was inserted in this function for the requested path")
    }

    /// Resolves an expression as if it were in the context
    /// of the given model.
    pub(super) fn resolve_expr_in_model(
        &mut self,
        expr_ast: &ast::ExprNode,
        model_path: &ModelPath,
    ) -> Result<output::ir::Expr, Vec<VariableResolutionError>> {
        resolver::resolve_expr_in_model(expr_ast, model_path, self)
    }

    /// Resolves an AST unit expression into a composite unit representation.
    pub(super) fn resolve_unit(
        &mut self,
        unit_ast: &ast::UnitExprNode,
    ) -> Result<ir::CompositeUnit, Vec<UnitResolutionError>> {
        resolver::resolve_unit(unit_ast, self)
    }
}

impl resolver::ExternalResolutionContext for Runtime {
    fn has_builtin_value(&self, identifier: &ast::Identifier) -> bool {
        self.builtins.has_builtin_value(identifier.as_str())
    }

    fn has_builtin_function(&self, identifier: &ast::Identifier) -> bool {
        self.builtins.has_builtin_function(identifier.as_str())
    }

    fn get_builtin_value_names(&self) -> impl Iterator<Item = &BuiltinValueName> {
        self.builtins.builtin_values().map(|(name, _)| name)
    }

    fn get_builtin_function_names(&self) -> impl Iterator<Item = &BuiltinFunctionName> {
        self.builtins.builtin_functions().map(|(name, _)| name)
    }

    fn has_builtin_unit(&self, name: &str) -> bool {
        let name = UnitBaseName::from(name);
        self.builtins.get_unit(&name).is_some()
    }

    fn available_prefixes(&self) -> impl Iterator<Item = (&UnitPrefix, f64)> {
        self.builtins.builtin_prefixes()
    }

    fn unit_supports_si_prefixes(&self, name: &UnitBaseName) -> bool {
        self.builtins.unit_supports_si_prefixes(name)
    }

    fn load_ast(
        &mut self,
        path: &ModelPath,
    ) -> LoadResult<&ast::ModelNode, resolver::AstLoadingFailedError> {
        self.load_ast_internal(path)
            .as_ref()
            .map_err(|_e| resolver::AstLoadingFailedError)
    }

    #[cfg(feature = "python")]
    fn load_python_import<'context>(
        &'context mut self,
        python_path: &PythonPath,
    ) -> Result<IndexSet<&'context PyFunctionName>, resolver::PythonImportLoadingFailedError> {
        self.load_python_import_internal(python_path)
            .as_ref()
            .ok()
            .map(|functions| functions.get_function_names().collect())
            .ok_or(resolver::PythonImportLoadingFailedError)
    }

    /// Resolver never calls this when the `python` feature is disabled; the stub satisfies
    /// [`ExternalResolutionContext`](resolver::ExternalResolutionContext).
    #[cfg(not(feature = "python"))]
    fn load_python_import<'context>(
        &'context mut self,
        _python_path: &PythonPath,
    ) -> Result<IndexSet<&'context PyFunctionName>, resolver::PythonImportLoadingFailedError> {
        Err(resolver::PythonImportLoadingFailedError)
    }

    fn get_preloaded_models(
        &self,
    ) -> impl Iterator<
        Item = (
            ModelPath,
            &LoadResult<ir::Model, resolver::ResolutionErrorCollection>,
        ),
    > {
        self.ir_cache
            .iter()
            .map(|(path, result)| (path.clone(), result))
    }
}
