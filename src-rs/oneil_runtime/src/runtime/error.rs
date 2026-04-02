//! Error reporting for models and parameters.

use indexmap::{IndexMap, IndexSet};
use oneil_output::{EvalError, Model, ModelEvalErrors};
use oneil_resolver::{
    ResolutionErrorCollection,
    error::{
        ModelImportResolutionError, ParameterResolutionError, PythonImportResolutionError,
        VariableResolutionError,
    },
};
use oneil_shared::{
    error::OneilDiagnostic,
    load_result::LoadResult,
    paths::{ModelPath, PythonPath},
    symbols::{ParameterName, ReferenceName, TestIndex},
};

use super::Runtime;
#[cfg(feature = "python")]
use crate::error::PythonImportError;
use crate::output::error::{ModelError, RuntimeErrors};

impl Runtime {
    /// Returns all diagnostics for the given model, as well as any referenced
    /// models that have issues (errors and evaluation warnings).
    ///
    /// Source or parsing failures are reported as a [`ModelError::FileError`].
    /// Resolution or evaluation failures are reported as [`ModelError::EvalErrors`].
    ///
    /// If `include_indirect_errors` is true, then errors from models that are referenced
    /// by the model are always included, regardless of whether they are referenced directly.
    ///
    /// For example, imagine there is an `x` in `model_a` that references `y` in `model_b`. Neither of
    /// these parameters have errors, but `model_b` has a parameter `z` that is used in a test, and both
    /// `z` and the test have errors. If `include_indirect_errors` is true, then the errors from `model_b`
    /// will be included in the errors for `model_a`. If `include_indirect_errors` is false, then the errors from `model_b`
    /// will not be included in the errors for `model_a` since there is no direct reference to `z` or to the test.
    #[must_use]
    pub(super) fn get_model_diagnostics(
        &self,
        model_path: &ModelPath,
        include_indirect_errors: bool,
    ) -> RuntimeErrors {
        let path_buf = model_path.clone().into_path_buf();

        // Handle source errors
        //
        // If the source failed to load, then there can be no
        // other errors, so we return early
        let Some(source_entry) = self.source_cache.get_entry(&model_path.into()) else {
            return RuntimeErrors::default();
        };

        let source = match source_entry {
            Ok(source) => source,
            Err(source_err) => {
                let mut errors = RuntimeErrors::default();

                errors.add_model_error(
                    model_path.clone(),
                    ModelError::FileError(vec![OneilDiagnostic::from_error(source_err, path_buf)]),
                );

                return errors;
            }
        };

        // Get the AST errors, if any
        let Some(ast_entry) = self.ast_cache.get_entry(model_path) else {
            return RuntimeErrors::default();
        };

        let ast_errors = match ast_entry {
            LoadResult::Failure => return RuntimeErrors::default(),
            LoadResult::Partial(_, parser_errors) => {
                let errors: Vec<OneilDiagnostic> = parser_errors
                    .iter()
                    .map(|e| OneilDiagnostic::from_error_with_source(e, path_buf.clone(), source))
                    .collect();

                Some(errors)
            }
            LoadResult::Success(_) => None,
        };

        // get the IR errors, if any
        let ir_errors = self
            .ir_cache
            .get_entry(model_path)
            .and_then(|entry| entry.error())
            .map(|errors| collect_ir_errors(errors, model_path, source, include_indirect_errors));

        let eval_entry = self.eval_cache.get_entry(model_path);
        let eval_errors = eval_entry
            .and_then(|entry| entry.error())
            .map(|errors| collect_eval_errors(errors, model_path, source, include_indirect_errors));

        let eval_model = eval_entry.and_then(|entry| entry.value());
        let eval_warning_diagnostics =
            eval_model.map(|model| extract_eval_warning_diagnostics(model, model_path, source));

        let merged = merge_ir_eval_diagnostics(ir_errors, eval_errors, eval_warning_diagnostics);

        let MergedErrors {
            models_with_errors,
            python_imports_with_errors,
            model_import_errors,
            python_import_errors,
            parameter_errors,
            test_errors,
        } = merged;

        #[cfg(not(feature = "python"))]
        let _ = python_imports_with_errors;

        let mut errors = RuntimeErrors::new();

        // add the errors for models that are referenced
        for model_path in models_with_errors {
            let model_errors = self.get_model_diagnostics(&model_path, include_indirect_errors);
            errors.extend(model_errors);
        }

        // add the errors for Python imports that are referenced
        #[cfg(feature = "python")]
        for python_import_path in python_imports_with_errors {
            let python_import_errors = self.get_python_import_errors(&python_import_path);
            errors.extend(python_import_errors);
        }

        if let Some(ast_errors) = ast_errors {
            // if there are AST errors, add them as a file error
            errors.add_model_error(model_path.clone(), ModelError::FileError(ast_errors));
        } else if !model_import_errors.is_empty()
            || !python_import_errors.is_empty()
            || !parameter_errors.is_empty()
            || !test_errors.is_empty()
        {
            // if there are other errors, add them as a model error
            errors.add_model_error(
                model_path.clone(),
                ModelError::EvalErrors {
                    model_import_errors: Box::new(model_import_errors),
                    python_import_errors: Box::new(python_import_errors),
                    parameter_errors: Box::new(parameter_errors),
                    test_errors: Box::new(test_errors),
                },
            );
        }

        errors
    }

    /// Returns errors for the given Python import path.
    ///
    /// If the source failed to load or the Python module failed to load (e.g. file not found or load error),
    /// returns a [`RuntimeErrors`] with [`ModelError::FileError`] entries for each.
    #[must_use]
    #[cfg(feature = "python")]
    pub(super) fn get_python_import_errors(
        &self,
        python_import_path: &PythonPath,
    ) -> RuntimeErrors {
        let path_buf = python_import_path.clone().into_path_buf();
        let mut errors = RuntimeErrors::new();

        if let Some(Err(source_err)) = self.source_cache.get_entry(&python_import_path.into()) {
            errors.add_python_import_error(
                python_import_path.clone(),
                OneilDiagnostic::from_error(source_err, path_buf.clone()),
            );
        }

        if let Some(Err(load_err)) = self.python_import_cache.get_entry(python_import_path)
            && let PythonImportError::LoadFailed(load_err) = load_err
        {
            errors.add_python_import_error(
                python_import_path.clone(),
                OneilDiagnostic::from_error(load_err, path_buf),
            );
        }

        errors
    }
}

/// Result of collecting errors from IR resolution.
#[expect(
    clippy::struct_field_names,
    reason = "removing 'errors' might be confusing"
)]
#[derive(Debug)]
struct IrErrorsResult {
    /// Model paths that have errors (for recursive collection).
    models_with_errors: IndexSet<ModelPath>,
    /// Python import paths that have errors (for recursive collection).
    python_imports_with_errors: IndexSet<PythonPath>,
    /// Model import resolution errors by reference name.
    model_import_errors: IndexMap<ReferenceName, OneilDiagnostic>,
    /// Python import resolution errors by path.
    python_import_errors: IndexMap<PythonPath, OneilDiagnostic>,
    /// Parameter resolution errors by parameter name.
    parameter_errors: IndexMap<ParameterName, Vec<OneilDiagnostic>>,
    /// Test resolution errors.
    test_errors: IndexMap<TestIndex, Vec<OneilDiagnostic>>,
}

/// Collects resolution errors from IR into structured error data and model/python path sets.
///
/// See [`Runtime::get_model_diagnostics`] for more details on the `include_indirect_errors` parameter.
fn collect_ir_errors(
    errors: &ResolutionErrorCollection,
    path: &ModelPath,
    source: &str,
    include_indirect_errors: bool,
) -> IrErrorsResult {
    let path_buf = path.clone().into_path_buf();

    // collect model import errors
    let mut model_import_errors = IndexMap::new();
    let mut models_with_errors = IndexSet::new();

    if include_indirect_errors {
        for (ref_name, (_submodel_name, ref_error)) in errors.get_model_import_resolution_errors() {
            if let Some(model_path) = get_model_path_from_model_import_error(ref_error) {
                models_with_errors.insert(model_path);
            }

            let error =
                OneilDiagnostic::from_error_with_source(ref_error, path_buf.clone(), source);
            model_import_errors.insert(ref_name.clone(), error);
        }
    }

    // collect Python import errors
    let mut python_import_errors = IndexMap::new();
    let mut python_imports_with_errors = IndexSet::new();
    for (python_path, err) in errors.get_python_import_resolution_errors() {
        if let Some(python_path) = get_python_path_from_python_import_error(err) {
            python_imports_with_errors.insert(python_path);
        }

        let error = OneilDiagnostic::from_error_with_source(err, path_buf.clone(), source);
        python_import_errors.insert(python_path.clone(), error);
    }

    let has_python_import_errors = !python_import_errors.is_empty();

    // collect parameter errors
    let mut parameter_errors = IndexMap::new();
    for (param_name, param_errs) in errors.get_parameter_resolution_errors() {
        let models_with_errors_in_param: IndexSet<_> = param_errs
            .iter()
            .filter_map(|error| {
                if let ParameterResolutionError::VariableResolution(
                    VariableResolutionError::ModelHasError { path, .. },
                ) = error
                {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();
        models_with_errors.extend(models_with_errors_in_param);

        let oneil_errors: Vec<OneilDiagnostic> = param_errs
            .iter()
            .filter(|e| !(has_python_import_errors && is_undefined_function_error(e)))
            .map(|e| OneilDiagnostic::from_error_with_source(e, path_buf.clone(), source))
            .collect();
        parameter_errors.insert(param_name.clone(), oneil_errors);
    }

    // collect test errors
    let mut test_errors = IndexMap::new();
    for (test_index, test_errs) in errors.get_test_resolution_errors() {
        let models_with_errors_in_test: IndexSet<_> = test_errs
            .iter()
            .filter_map(|error| {
                if let VariableResolutionError::ModelHasError { path, .. } = error {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();
        models_with_errors.extend(models_with_errors_in_test);

        let oneil_errors: Vec<OneilDiagnostic> = test_errs
            .iter()
            .map(|e| OneilDiagnostic::from_error_with_source(e, path_buf.clone(), source))
            .collect();
        test_errors.insert(*test_index, oneil_errors);
    }

    IrErrorsResult {
        models_with_errors,
        python_imports_with_errors,
        model_import_errors,
        python_import_errors,
        parameter_errors,
        test_errors,
    }
}

const fn is_undefined_function_error(error: &ParameterResolutionError) -> bool {
    matches!(
        error,
        ParameterResolutionError::VariableResolution(
            VariableResolutionError::UndefinedFunction { .. }
        )
    )
}

/// Result of collecting errors from evaluation.
#[expect(
    clippy::struct_field_names,
    reason = "removing 'errors' might be confusing"
)]
#[derive(Debug)]
struct EvalErrorsResult {
    /// Model paths that have errors (for recursive collection).
    models_with_errors: IndexSet<ModelPath>,
    /// Parameter evaluation errors by parameter name.
    parameter_errors: IndexMap<ParameterName, Vec<OneilDiagnostic>>,
    /// Test evaluation errors.
    test_errors: IndexMap<TestIndex, Vec<OneilDiagnostic>>,
}

/// Collects evaluation errors into structured error data and model path set.
///
/// See [`Runtime::get_model_diagnostics`] for more details on the `include_indirect_errors` parameter.
fn collect_eval_errors(
    errors: &ModelEvalErrors,
    path: &ModelPath,
    source: &str,
    include_indirect_errors: bool,
) -> EvalErrorsResult {
    let path_buf = path.clone().into_path_buf();

    let mut models_with_errors = IndexSet::new();

    if include_indirect_errors {
        for reference_path in &errors.references {
            models_with_errors.insert(reference_path.clone());
        }
    }

    let mut parameter_errors = IndexMap::new();
    for (name, param_errs) in &errors.parameters {
        let models_with_errors_in_param: IndexSet<_> = param_errs
            .iter()
            .filter_map(|error| {
                if let EvalError::ParameterHasError { model_path, .. } = error {
                    model_path.clone()
                } else {
                    None
                }
            })
            .collect();
        models_with_errors.extend(models_with_errors_in_param);

        let oneil_errors: Vec<OneilDiagnostic> = param_errs
            .iter()
            .map(|e| OneilDiagnostic::from_error_with_source(e, path_buf.clone(), source))
            .collect();
        parameter_errors.insert(name.clone(), oneil_errors);
    }

    let mut test_errors = IndexMap::new();
    for (test_index, test_errs) in &errors.tests {
        let mut test_errors_in_test = Vec::new();

        for test_err in test_errs {
            if let EvalError::ParameterHasError { model_path, .. } = test_err
                && let Some(p) = model_path
            {
                models_with_errors.insert(p.clone());
            }

            let error = OneilDiagnostic::from_error_with_source(test_err, path_buf.clone(), source);
            test_errors_in_test.push(error);
        }

        test_errors.insert(*test_index, test_errors_in_test);
    }

    EvalErrorsResult {
        models_with_errors,
        parameter_errors,
        test_errors,
    }
}

/// Diagnostics produced from [`Model`] evaluation warnings before they are merged with errors.
#[derive(Debug, Default)]
struct EvalWarningDiagnostics {
    parameter_warnings: IndexMap<ParameterName, Vec<OneilDiagnostic>>,
    test_warnings: IndexMap<TestIndex, Vec<OneilDiagnostic>>,
}

/// Builds [`EvalWarningDiagnostics`] from an evaluated model's parameter and test warning lists.
fn extract_eval_warning_diagnostics(
    model: &Model,
    path: &ModelPath,
    source: &str,
) -> EvalWarningDiagnostics {
    let path_buf = path.clone().into_path_buf();
    let mut out = EvalWarningDiagnostics::default();

    for (name, parameter) in &model.parameters {
        if parameter.warnings.is_empty() {
            continue;
        }

        let diags: Vec<OneilDiagnostic> = parameter
            .warnings
            .iter()
            .map(|w| OneilDiagnostic::from_error_with_source(w, path_buf.clone(), source))
            .collect();

        out.parameter_warnings.insert(name.clone(), diags);
    }

    for (test_index, test) in &model.tests {
        if test.warnings.is_empty() {
            continue;
        }

        let diags: Vec<OneilDiagnostic> = test
            .warnings
            .iter()
            .map(|w| OneilDiagnostic::from_error_with_source(w, path_buf.clone(), source))
            .collect();

        out.test_warnings.insert(*test_index, diags);
    }

    out
}

/// Result of merging IR resolution errors, evaluation errors, and evaluation warnings.
#[expect(
    clippy::struct_field_names,
    reason = "removing 'errors' might be confusing"
)]
#[derive(Debug)]
struct MergedErrors {
    /// Model paths that have errors (for recursive collection).
    pub models_with_errors: IndexSet<ModelPath>,
    /// Python import paths that have errors (for recursive collection).
    pub python_imports_with_errors: IndexSet<PythonPath>,
    /// Model import errors by reference name.
    pub model_import_errors: IndexMap<ReferenceName, OneilDiagnostic>,
    /// Python import errors by path.
    pub python_import_errors: IndexMap<PythonPath, OneilDiagnostic>,
    /// Parameter diagnostics (resolution and evaluation errors, plus evaluation warnings).
    pub parameter_errors: IndexMap<ParameterName, Vec<OneilDiagnostic>>,
    /// Test diagnostics (resolution and evaluation errors, plus evaluation warnings).
    pub test_errors: IndexMap<TestIndex, Vec<OneilDiagnostic>>,
}

/// Merges IR resolution errors, evaluation errors, and evaluation warnings.
///
/// When both are present, model paths are unioned. Parameter diagnostics are built with eval then IR
/// in the iterator so IR overwrites eval for duplicate keys; test diagnostics use IR then eval so
/// eval overwrites IR for duplicate keys.
fn merge_ir_eval_diagnostics(
    ir_errors: Option<IrErrorsResult>,
    eval_errors: Option<EvalErrorsResult>,
    eval_warning_diagnostics: Option<EvalWarningDiagnostics>,
) -> MergedErrors {
    let mut merged = match (ir_errors, eval_errors) {
        (Some(ir), Some(eval)) => MergedErrors {
            models_with_errors: ir
                .models_with_errors
                .union(&eval.models_with_errors)
                .cloned()
                .collect(),
            python_imports_with_errors: ir.python_imports_with_errors,
            model_import_errors: ir.model_import_errors,
            python_import_errors: ir.python_import_errors,
            // note that in the case of the same parameter/test having errors in both IR and eval,
            // the IR errors are preferred because `ir` comes later in the chain
            parameter_errors: eval
                .parameter_errors
                .into_iter()
                .chain(ir.parameter_errors)
                .collect(),
            test_errors: ir.test_errors.into_iter().chain(eval.test_errors).collect(),
        },

        (Some(ir), None) => MergedErrors {
            models_with_errors: ir.models_with_errors,
            python_imports_with_errors: ir.python_imports_with_errors,
            model_import_errors: ir.model_import_errors,
            python_import_errors: ir.python_import_errors,
            parameter_errors: ir.parameter_errors,
            test_errors: ir.test_errors,
        },

        (None, Some(eval)) => MergedErrors {
            models_with_errors: eval.models_with_errors,
            python_imports_with_errors: IndexSet::new(),
            model_import_errors: IndexMap::new(),
            python_import_errors: IndexMap::new(),
            parameter_errors: eval.parameter_errors,
            test_errors: eval.test_errors,
        },

        (None, None) => MergedErrors {
            models_with_errors: IndexSet::new(),
            python_imports_with_errors: IndexSet::new(),
            model_import_errors: IndexMap::new(),
            python_import_errors: IndexMap::new(),
            parameter_errors: IndexMap::new(),
            test_errors: IndexMap::new(),
        },
    };

    // add the warnings to the merged errors if they are present
    let Some(warnings) = eval_warning_diagnostics else {
        return merged;
    };

    for (name, parameter) in &warnings.parameter_warnings {
        merged
            .parameter_errors
            .entry(name.clone())
            .or_default()
            .extend(parameter.clone());
    }

    for (test_index, test) in &warnings.test_warnings {
        merged
            .test_errors
            .entry(*test_index)
            .or_default()
            .extend(test.clone());
    }

    merged
}

/// Returns the model path from a model import error when available.
fn get_model_path_from_model_import_error(err: &ModelImportResolutionError) -> Option<ModelPath> {
    match err {
        ModelImportResolutionError::ModelHasError { model_path, .. } => Some(model_path.clone()),

        ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path, ..
        } => Some(parent_model_path.clone()),

        ModelImportResolutionError::ParentModelHasError { .. }
        | ModelImportResolutionError::DuplicateSubmodel { .. }
        | ModelImportResolutionError::DuplicateReference { .. } => None,
    }
}

/// Returns the Python path from a Python import error when available.
fn get_python_path_from_python_import_error(
    err: &PythonImportResolutionError,
) -> Option<PythonPath> {
    match err {
        PythonImportResolutionError::FailedValidation { python_path, .. } => {
            Some(python_path.clone())
        }

        PythonImportResolutionError::DuplicateImport { .. }
        | PythonImportResolutionError::PythonNotEnabled { .. } => None,
    }
}
