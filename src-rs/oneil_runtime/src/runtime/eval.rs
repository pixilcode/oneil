//! Model evaluation for the runtime.

use std::path::PathBuf;

use indexmap::IndexMap;
use oneil_eval::{self as eval, IrLoadError};
use oneil_output::{Unit, Value};
use oneil_shared::{
    EvalInstanceKey,
    error::OneilError,
    load_result::LoadResult,
    paths::ModelPath,
    span::Span,
    symbols::{BuiltinFunctionName, BuiltinValueName, UnitBaseName, UnitPrefix},
};
#[cfg(feature = "python")]
use oneil_shared::{paths::PythonPath, symbols::PyFunctionName};

use oneil_resolver as resolver;

use super::Runtime;
use crate::output::{self, error::RuntimeErrors, ir};

type EvalModelAndExpressionsResult<'runtime, 'expr> = (
    Option<(
        output::reference::ModelReference<'runtime>,
        IndexMap<&'expr str, Value>,
    )>,
    RuntimeErrors,
    Vec<OneilError>,
);

impl Runtime {
    /// Evaluates a model with an optional design file applied.
    ///
    /// When a design path is provided, the design bundle from that file is applied
    /// to the model being evaluated. The design file must target the model being
    /// evaluated (i.e., contain `design <model_name>`).
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeErrors`] if the model or design file could not be evaluated.
    pub fn eval_model(
        &mut self,
        path: &ModelPath,
        design_path: Option<&ModelPath>,
    ) -> (Option<output::reference::ModelReference<'_>>, RuntimeErrors) {
        // Evaluate the model (with optional design) - populates caches
        self.eval_model_internal(path, design_path);

        // Look up the model reference from cache
        let model_opt = self
            .eval_cache
            .get_entry(path)
            .and_then(LoadResult::value)
            .map(|model| output::reference::ModelReference::new(model, &self.eval_cache));

        let include_indirect_errors = true;
        let mut errors = self.get_model_errors(path, include_indirect_errors);

        // Also include design file errors if present
        if let Some(design_path) = design_path {
            let design_errors = self.get_model_errors(design_path, include_indirect_errors);
            errors.extend(design_errors);
        }

        (model_opt, errors)
    }

    /// Evaluates a model (with optional design) and a list of expressions in the context of
    /// the given model and returns the result.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeErrors`] (via [`get_model_errors`](super::Runtime::get_model_errors)) if the model could not be evaluated.
    /// Returns [`OneilError`]s if the expressions could not be evaluated.
    pub fn eval_model_and_expressions<'runtime, 'expr>(
        &'runtime mut self,
        path: &ModelPath,
        design_path: Option<&ModelPath>,
        expressions: &'expr [String],
    ) -> EvalModelAndExpressionsResult<'runtime, 'expr> {
        // Evaluate the model (with optional design) - populates caches
        self.eval_model_internal(path, design_path);

        // Evaluate the expressions
        let (expr_results, expr_errors) = self.eval_expressions_internal(expressions, path);

        // Look up the model reference from cache
        let model_opt = self
            .eval_cache
            .get_entry(path)
            .and_then(LoadResult::value)
            .map(|model| output::reference::ModelReference::new(model, &self.eval_cache));

        let result = model_opt.map(|model| (model, expr_results));

        let include_indirect_errors = true;
        let mut model_errors = self.get_model_errors(path, include_indirect_errors);

        // Also include design file errors if present
        if let Some(design_path) = design_path {
            let design_errors = self.get_model_errors(design_path, include_indirect_errors);
            model_errors.extend(design_errors);
        }

        (result, model_errors, expr_errors)
    }

    /// Internal evaluation that populates caches without returning references.
    fn eval_model_internal(&mut self, path: &ModelPath, design_path: Option<&ModelPath>) {
        // Load the model IR
        self.load_ir_internal(path);

        // If design path provided, load it (without overwriting the target model) and
        // build a runtime DesignApplication targeting the root model.
        let runtime_designs: Vec<oneil_ir::DesignApplication> =
            if let Some(design_path) = design_path {
                let design_results = resolver::load_model(design_path, self);
                for (model_path_result, result) in design_results {
                    // Skip if this is not the design file itself (avoid overwriting target models)
                    if model_path_result != *design_path {
                        continue;
                    }
                    let (model, model_errors) = result.into_parts();
                    if model_errors.is_empty() {
                        self.ir_cache
                            .insert(model_path_result, LoadResult::success(model));
                    } else {
                        self.ir_cache
                            .insert(model_path_result, LoadResult::partial(model, model_errors));
                    }
                }

                vec![oneil_ir::DesignApplication {
                    design_path: design_path.clone(),
                    applied_to: None,
                    span: oneil_shared::span::Span::empty(oneil_shared::span::SourceLocation {
                        offset: 0,
                        line: 1,
                        column: 1,
                    }),
                }]
            } else {
                Vec::new()
            };

        // Evaluate the model and its dependencies
        let eval_result = eval::eval_model_with_designs(path, &runtime_designs, self);

        for (instance_key, maybe_partial) in eval_result {
            match maybe_partial.into_result() {
                Ok(model) => {
                    self.eval_cache
                        .insert(instance_key.clone(), LoadResult::success(model));
                }
                Err(partial) => {
                    self.eval_cache.insert(
                        instance_key,
                        LoadResult::partial(partial.partial_result, partial.error_collection),
                    );
                }
            }
        }
    }

    /// Evaluates a list of expressions in the context of
    /// the given model and returns the results.
    fn eval_expressions_internal<'expr>(
        &mut self,
        expressions: &'expr [String],
        model_path: &ModelPath,
    ) -> (IndexMap<&'expr str, Value>, Vec<OneilError>) {
        let mut results = IndexMap::new();
        let mut errors = Vec::new();

        for (index, expression) in expressions.iter().enumerate() {
            // a pseudo path for the expression, to be used for error reporting
            // this is not a real path, but it is a unique path for the expression
            let pseudo_path = format!("/oneil-eval/expr-{index}");
            let pseudo_path = PathBuf::from(pseudo_path);

            let expr_ast = match Self::parse_expression(expression) {
                Ok(expr_ast) => expr_ast,
                Err(error) => {
                    let oneil_error =
                        OneilError::from_error_with_source(&error, pseudo_path, expression);

                    errors.push(oneil_error);

                    continue;
                }
            };

            let expr_ir = match self.resolve_expr_in_model(&expr_ast, model_path) {
                Ok(expr_ir) => expr_ir,
                Err(resolution_errors) => {
                    let oneil_errors = resolution_errors.into_iter().map(|error| {
                        OneilError::from_error_with_source(&error, pseudo_path.clone(), expression)
                    });

                    errors.extend(oneil_errors);

                    continue;
                }
            };

            let eval_result = match self.eval_expr_in_model(&expr_ir, model_path) {
                Ok(eval_result) => eval_result,
                Err(eval_errors) => {
                    let oneil_errors = eval_errors.into_iter().map(|error| {
                        OneilError::from_error_with_source(&error, pseudo_path.clone(), expression)
                    });

                    errors.extend(oneil_errors);

                    continue;
                }
            };

            results.insert(expression.as_str(), eval_result);
        }

        (results, errors)
    }

    /// Evaluates an expression as if it were in the context
    /// of the given model.
    fn eval_expr_in_model(
        &mut self,
        expr_ir: &output::ir::Expr,
        model_path: &ModelPath,
    ) -> Result<Value, Vec<eval::EvalError>> {
        eval::eval_expr_in_model(expr_ir, model_path, self)
    }
}

impl eval::ExternalEvaluationContext for Runtime {
    fn lookup_ir(&self, path: &ModelPath) -> Option<LoadResult<&ir::Model, IrLoadError>> {
        let entry = self.ir_cache.get_entry(path)?;
        let result = entry.as_ref().map_err(|_error| eval::IrLoadError);

        Some(result)
    }

    fn lookup_builtin_variable(&self, name: &BuiltinValueName) -> Option<&Value> {
        self.builtins.get_value(name)
    }

    fn evaluate_builtin_function(
        &self,
        name: &BuiltinFunctionName,
        name_span: Span,
        args: Vec<(Value, Span)>,
    ) -> Option<Result<Value, Vec<eval::EvalError>>> {
        let builtin = self.builtins.get_function(name)?;
        Some(builtin.call(name_span, args))
    }

    #[cfg(feature = "python")]
    fn evaluate_imported_function(
        &self,
        python_path: &PythonPath,
        identifier: &PyFunctionName,
        function_call_span: Span,
        args: Vec<(output::Value, Span)>,
    ) -> Option<Result<output::Value, Box<eval::EvalError>>> {
        self.evaluate_python_function(python_path, identifier, function_call_span, args)
    }

    fn lookup_unit(&self, name: &UnitBaseName) -> Option<&Unit> {
        self.builtins.get_unit(name)
    }

    fn lookup_prefix(&self, name: &UnitPrefix) -> Option<f64> {
        self.builtins.get_prefix(name)
    }

    fn get_preloaded_models(
        &self,
    ) -> impl Iterator<
        Item = (
            EvalInstanceKey,
            &LoadResult<output::Model, eval::EvalErrors>,
        ),
    > {
        self.eval_cache
            .iter()
            .map(|(key, result)| (key.clone(), result))
    }
}
