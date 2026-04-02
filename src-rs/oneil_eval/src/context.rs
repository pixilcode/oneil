use indexmap::{IndexMap, IndexSet};

use oneil_ir as ir;
use oneil_output::{self as output, EvalError};
use oneil_shared::{
    load_result::LoadResult,
    partial::MaybePartialResult,
    paths::{ModelPath, PythonPath},
    span::Span,
    symbols::{
        BuiltinFunctionName, BuiltinValueName, ParameterName, PyFunctionName, ReferenceName,
        SubmodelName, TestIndex, UnitBaseName, UnitPrefix,
    },
};

/// Error indicating that an IR model could not be loaded.
#[derive(Debug, Clone, Copy)]
pub struct IrLoadError;

/// Context provided by the runtime for resolving IR, builtins, and units during evaluation.
pub trait ExternalEvaluationContext {
    /// Returns the IR model at the given path if it has been loaded.
    fn lookup_ir(&self, path: &ModelPath) -> Option<LoadResult<&ir::Model, IrLoadError>>;

    /// Returns the value of a builtin variable by identifier, if it exists.
    fn lookup_builtin_variable(&self, name: &BuiltinValueName) -> Option<&output::Value>;

    /// Evaluates a builtin function by identifier with the given arguments, if it exists.
    ///
    /// If the function does not exist, returns `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if there was an error evaluating the builtin function.
    fn evaluate_builtin_function(
        &self,
        name: &BuiltinFunctionName,
        name_span: Span,
        args: Vec<(output::Value, Span)>,
    ) -> Option<Result<output::Value, Vec<EvalError>>>;

    /// Evaluates an imported function by identifier with the given arguments, if it exists.
    ///
    /// If the function does not exist, returns `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if there was an error evaluating the imported function.
    #[cfg(feature = "python")]
    fn evaluate_imported_function(
        &self,
        python_path: &PythonPath,
        identifier: &PyFunctionName,
        function_call_span: Span,
        args: Vec<(output::Value, Span)>,
    ) -> Option<Result<output::Value, Box<EvalError>>>;

    /// Returns a unit by name if it is defined in the builtin context.
    fn lookup_unit(&self, name: &UnitBaseName) -> Option<&output::Unit>;

    /// Returns a prefix by name if it is defined in the builtin context.
    fn lookup_prefix(&self, name: &UnitPrefix) -> Option<f64>;

    /// Returns pre-loaded evaluated models.
    fn get_preloaded_models(
        &self,
    ) -> impl Iterator<
        Item = (
            ModelPath,
            &LoadResult<output::Model, output::ModelEvalErrors>,
        ),
    >;
}

/// Represents a model in progress of being evaluated.
#[derive(Debug, Clone)]
struct ModelInProgress {
    parameters: IndexMap<ParameterName, Result<output::Parameter, Vec<EvalError>>>,
    submodels: IndexMap<SubmodelName, ReferenceName>,
    references: IndexMap<ReferenceName, ModelPath>,
    references_with_errors: IndexSet<ModelPath>,
    tests: IndexMap<TestIndex, Result<output::Test, Vec<EvalError>>>,
}

impl ModelInProgress {
    /// Creates a new empty model.
    pub fn new() -> Self {
        Self {
            parameters: IndexMap::new(),
            submodels: IndexMap::new(),
            references: IndexMap::new(),
            references_with_errors: IndexSet::new(),
            tests: IndexMap::new(),
        }
    }
}

impl Default for ModelInProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluation context that tracks models, their parameters, dependencies, and builtin functions.
///
/// The context maintains state during evaluation, including:
/// - Evaluated models and their parameters
/// - Active models
/// - External context
#[derive(Debug)]
pub struct EvalContext<'external, E: ExternalEvaluationContext> {
    models: IndexMap<ModelPath, ModelInProgress>,
    active_models: Vec<ModelPath>,
    external_context: &'external mut E,
}

impl<'external, E: ExternalEvaluationContext> EvalContext<'external, E> {
    /// Creates a new evaluation context with the given builtin functions.
    #[must_use]
    pub fn new(external_context: &'external mut E) -> Self {
        Self {
            models: IndexMap::new(),
            active_models: Vec::new(),
            external_context,
        }
    }

    /// Creates a new evaluation context with the given pre-loaded models.
    #[must_use]
    pub fn with_preloaded_models(external_context: &'external mut E) -> Self {
        let models = external_context
            .get_preloaded_models()
            .map(|(path, result)| {
                let model = match result {
                    LoadResult::Success(model) => ModelInProgress {
                        parameters: model
                            .parameters
                            .iter()
                            .map(|(name, parameter)| (name.clone(), Ok(parameter.clone())))
                            .collect(),
                        submodels: model.submodels.clone(),
                        references: model.references.clone(),
                        references_with_errors: IndexSet::new(),
                        tests: model
                            .tests
                            .iter()
                            .map(|(index, test)| (*index, Ok(test.clone())))
                            .collect(),
                    },

                    LoadResult::Partial(model, errors) => ModelInProgress {
                        parameters: model
                            .parameters
                            .iter()
                            .map(|(name, parameter)| (name.clone(), Ok(parameter.clone())))
                            .chain(
                                errors
                                    .parameters
                                    .iter()
                                    .map(|(name, errs)| (name.clone(), Err(errs.clone()))),
                            )
                            .collect(),

                        submodels: model.submodels.clone(),
                        references: model.references.clone(),
                        references_with_errors: errors.references.clone(),
                        tests: model
                            .tests
                            .iter()
                            .map(|(index, test)| (*index, Ok(test.clone())))
                            .chain(
                                errors
                                    .tests
                                    .iter()
                                    .map(|(index, errs)| (*index, Err(errs.clone()))),
                            )
                            .collect(),
                    },

                    LoadResult::Failure => ModelInProgress::new(),
                };

                (path, model)
            })
            .collect();

        Self {
            models,
            active_models: Vec::new(),
            external_context,
        }
    }

    /// Consumes the context and returns the accumulated models and errors.
    ///
    /// Each entry maps a model path to a [`MaybePartialResult`]: either a full
    /// success with the evaluated [`Model`], or a partial result (the model) and
    /// any [`ModelEvalErrors`] that occurred during evaluation (e.g. from parameters
    /// or tests that failed).
    #[must_use]
    pub fn into_result(
        self,
    ) -> IndexMap<ModelPath, MaybePartialResult<output::Model, output::ModelEvalErrors>> {
        let mut result = IndexMap::new();

        // for each model, collect the parameters and tests, and any errors
        for (path, model) in self.models {
            // collect the parameters and any errors
            let mut parameters = IndexMap::new();
            let mut parameter_errors = IndexMap::new();
            for (name, result) in model.parameters {
                match result {
                    Ok(param) => {
                        parameters.insert(name, param);
                    }

                    Err(errs) => {
                        parameter_errors.insert(name, errs);
                    }
                }
            }

            // collect the tests and any errors
            let mut tests = IndexMap::new();
            let mut test_errors = IndexMap::new();
            for (index, test) in model.tests {
                match test {
                    Ok(test) => {
                        tests.insert(index, test);
                    }
                    Err(errs) => {
                        test_errors.insert(index, errs);
                    }
                }
            }

            // create the output model
            let output_model = output::Model {
                path: path.clone(),
                submodels: model.submodels,
                references: model.references,
                parameters,
                tests,
            };

            if parameter_errors.is_empty()
                && test_errors.is_empty()
                && model.references_with_errors.is_empty()
            {
                result.insert(path, MaybePartialResult::ok(output_model));
            } else {
                result.insert(
                    path,
                    MaybePartialResult::err(
                        output_model,
                        output::ModelEvalErrors {
                            parameters: parameter_errors,
                            tests: test_errors,
                            references: model.references_with_errors,
                        },
                    ),
                );
            }
        }

        result
    }

    /// Looks up an IR model by path.
    ///
    /// # Panics
    ///
    /// Panics if the model is not found. This should never be the case.
    pub fn get_ir(&self, path: &ModelPath) -> LoadResult<ir::Model, IrLoadError> {
        self.external_context
            .lookup_ir(path)
            .expect("model should be found")
            // TODO: figure out how to get rid of this clone
            .map(ir::Model::clone)
    }

    /// Looks up the given builtin variable and returns the corresponding value.
    ///
    /// # Panics
    ///
    /// Panics if the builtin value is not defined. This should never be the case.
    /// If it is, then there is a bug either in the model resolver when it resolves builtin variables
    /// or in the builtin map when it defines the builtin values.
    #[must_use]
    pub fn lookup_builtin_variable(&self, name: &BuiltinValueName) -> output::Value {
        self.external_context
            .lookup_builtin_variable(name)
            .expect("builtin value should be defined (checked during resolution)")
            .clone()
    }

    /// Looks up a parameter value in the current model.
    ///
    /// # Panics
    ///
    /// Panics if no current model is set or if the parameter is not defined in the model.
    pub fn lookup_parameter_value(
        &self,
        parameter_name: &ParameterName,
        variable_span: Span,
    ) -> Result<output::Value, Vec<EvalError>> {
        let current_model = self
            .active_models
            .last()
            .expect("current model should be set when looking up a parameter");

        self.lookup_model_parameter_value_internal(
            current_model,
            parameter_name,
            variable_span,
            true,
        )
    }

    /// Looks up a parameter value in a specific model.
    pub fn lookup_model_parameter_value(
        &self,
        model: &ModelPath,
        parameter_name: &ParameterName,
        variable_span: Span,
    ) -> Result<output::Value, Vec<EvalError>> {
        self.lookup_model_parameter_value_internal(model, parameter_name, variable_span, false)
    }

    fn lookup_model_parameter_value_internal(
        &self,
        model_path: &ModelPath,
        parameter_name: &ParameterName,
        variable_span: Span,
        is_current_model: bool,
    ) -> Result<output::Value, Vec<EvalError>> {
        let model = self
            .models
            .get(model_path)
            .expect("current model should be created when set");

        model
            .parameters
            .get(parameter_name)
            .expect("parameter should be defined")
            .clone()
            .map(|parameter| parameter.value)
            .map_err(|_errors| {
                let model_path = if is_current_model {
                    None
                } else {
                    Some(model_path.clone())
                };

                vec![EvalError::ParameterHasError {
                    model_path,
                    parameter_name: parameter_name.clone(),
                    variable_span,
                }]
            })
    }

    /// Evaluates a builtin function with the given arguments.
    ///
    /// # Panics
    ///
    /// Panics if the builtin function is not defined. This should never be the case.
    pub fn evaluate_builtin_function(
        &self,
        name: &BuiltinFunctionName,
        name_span: Span,
        args: Vec<(output::Value, Span)>,
    ) -> Result<output::Value, Vec<EvalError>> {
        self.external_context
            .evaluate_builtin_function(name, name_span, args)
            .expect("builtin function should be defined (checked during resolution)")
    }

    /// Evaluates an imported function with the given arguments.
    pub fn evaluate_imported_function(
        &self,
        python_path: &PythonPath,
        name: &PyFunctionName,
        function_call_span: Span,
        args: Vec<(output::Value, Span)>,
    ) -> Result<output::Value, Box<EvalError>> {
        #[cfg(feature = "python")]
        {
            self.external_context
                .evaluate_imported_function(python_path, name, function_call_span, args)
                .expect("imported function should be defined (checked during resolution)")
        }

        #[cfg(not(feature = "python"))]
        {
            let _ = (self, python_path, name, args);
            Err(Box::new(EvalError::PythonNotEnabled {
                relevant_span: function_call_span,
            }))
        }
    }

    /// Looks up a unit by name.
    #[must_use]
    pub fn lookup_unit(&self, name: &UnitBaseName) -> Option<output::Unit> {
        self.external_context.lookup_unit(name).cloned()
    }

    /// Looks up a prefix by name.
    #[must_use]
    pub fn lookup_prefix(&self, name: &UnitPrefix) -> Option<f64> {
        self.external_context.lookup_prefix(name)
    }

    /// Pushes the active model for evaluation.
    ///
    /// Creates a new model entry if it doesn't exist.
    pub fn push_active_model(&mut self, model_path: ModelPath) {
        self.models.entry(model_path.clone()).or_default();

        self.active_models.push(model_path);
    }

    /// Clears the active model.
    pub fn pop_active_model(&mut self, model_path: &ModelPath) {
        assert_eq!(self.active_models.last(), Some(model_path));

        self.active_models.pop();
    }

    /// Adds a parameter evaluation result to the current model.
    ///
    /// # Panics
    ///
    /// Panics if no current model is set or if the current model was not created.
    pub fn add_parameter_result(
        &mut self,
        parameter_name: ParameterName,
        result: Result<output::Parameter, Vec<EvalError>>,
    ) {
        // TODO: Maybe use type state pattern to enforce this?
        let Some(current_model) = self.active_models.last() else {
            panic!("current model should be set when adding a parameter result");
        };

        let model = self
            .models
            .get_mut(current_model)
            .expect("current model should be created when set");

        model.parameters.insert(parameter_name, result);
    }

    /// Adds a submodel to the current model.
    ///
    /// # Panics
    ///
    /// Panics if no current model is set or if the current model was not created.
    pub(crate) fn add_submodel(
        &mut self,
        submodel_name: &SubmodelName,
        submodel_reference_name: &ReferenceName,
    ) {
        let Some(current_model) = self.active_models.last() else {
            panic!("current model should be set when adding a submodel");
        };

        let model = self
            .models
            .get_mut(current_model)
            .expect("current model should be created when set");

        model
            .submodels
            .insert(submodel_name.clone(), submodel_reference_name.clone());
    }

    /// Adds a reference to the current model.
    ///
    /// # Panics
    ///
    /// Panics if no current model is set or if the current model was not created.
    pub(crate) fn add_reference(
        &mut self,
        reference_name: &ReferenceName,
        reference_path: &ModelPath,
    ) {
        let Some(current_model) = self.active_models.last() else {
            panic!("current model should be set when adding a reference");
        };

        let model = self
            .models
            .get_mut(current_model)
            .expect("current model should be created when set");

        model
            .references
            .insert(reference_name.clone(), reference_path.clone());
    }

    /// Returns whether the model at the given path has any evaluation errors.
    ///
    /// A model has errors if any of its parameters failed to evaluate or any of its tests failed.
    #[must_use]
    pub fn reference_has_errors(&self, path: &ModelPath) -> bool {
        let Some(model) = self.models.get(path) else {
            return false;
        };
        let has_parameter_errors = model.parameters.values().any(Result::is_err);
        let has_test_errors = model.tests.iter().any(|(_, result)| result.is_err());
        let has_reference_errors = !model.references_with_errors.is_empty();

        has_parameter_errors || has_test_errors || has_reference_errors
    }

    /// Records that the given reference path has errors on the current active model.
    ///
    /// # Panics
    ///
    /// Panics if no current model is set or if the current model was not created.
    pub fn add_reference_error_to_active_model(&mut self, path: &ModelPath) {
        let Some(current_model) = self.active_models.last() else {
            panic!("current model should be set when adding a reference error");
        };

        let model = self
            .models
            .get_mut(current_model)
            .expect("current model should be created when set");

        model.references_with_errors.insert(path.clone());
    }

    /// Adds a test evaluation result to the current model.
    ///
    /// # Panics
    ///
    /// Panics if no current model is set or if the current model was not created.
    pub(crate) fn add_test_result(
        &mut self,
        test_index: TestIndex,
        test_result: Result<output::Test, Vec<EvalError>>,
    ) {
        let Some(current_model) = self.active_models.last() else {
            panic!("current model should be set when adding a test result");
        };

        let model = self
            .models
            .get_mut(current_model)
            .expect("current model should be created when set");

        model.tests.insert(test_index, test_result);
    }
}
