use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use oneil_ir as ir;
use oneil_shared::{
    partial::MaybePartialResult,
    paths::ModelPath,
    symbols::{BuiltinValueName, ReferenceName, TestIndex},
};
use oneil_shared::{span::Span, symbols::ParameterName};

use oneil_output::{
    self as output, BuiltinDependency, DependencySet, EvalError, EvalWarning, ExpectedType,
    ExternalDependency, Model, ModelEvalErrors, ParameterDependency, Value,
};

use crate::{
    context::{EvalContext, ExternalEvaluationContext},
    eval_expr, eval_parameter,
};

/// Evaluates the model at the given path and all its dependencies, returning a map of
/// path to evaluated model result for each model that was evaluated.
pub fn eval_model<E: ExternalEvaluationContext>(
    model_path: &ModelPath,
    external_context: &mut E,
) -> IndexMap<ModelPath, MaybePartialResult<Model, ModelEvalErrors>> {
    let mut context = EvalContext::new(external_context);

    eval_model_from_context(model_path, &mut context);

    context.into_result()
}

/// Evaluates a model and returns the context with the results of the model.
fn eval_model_from_context<E: ExternalEvaluationContext>(
    model_path: &ModelPath,
    context: &mut EvalContext<'_, E>,
) {
    // Set the current model
    context.push_active_model(model_path.clone());

    let model = context.get_ir(model_path);

    let Some(model) = model.value() else {
        return;
    };

    // Recursively evaluate references
    let references = model.get_references();
    for reference_import in references.values() {
        eval_model_from_context(reference_import.path(), context);
    }

    // Check for errors in references
    for reference_import in references.values() {
        if context.reference_has_errors(reference_import.path()) {
            context.add_reference_error_to_active_model(reference_import.path());
        }
    }

    // Bring references into scope
    for (reference_name, reference_import) in references {
        context.add_reference(reference_name, reference_import.path());
    }

    // Add submodels to the current model
    let submodels = model.get_submodels();
    for (submodel_name, submodel_import) in submodels {
        context.add_submodel(submodel_name, submodel_import.reference_name());
    }

    // Evaluate parameters
    let parameters = model.get_parameters();
    let evaluation_order = get_evaluation_order(parameters);

    for parameter_name in evaluation_order {
        let parameter = parameters
            .get(&parameter_name)
            .expect("parameter should exist because it comes from the keys of the parameters map");

        let value = eval_parameter::eval_parameter(parameter_name.clone(), parameter, context);

        let parameter_result = value.map(|value| {
            parameter_result_from(
                value.value,
                value.expr_span,
                value.warnings,
                parameter,
                context,
            )
        });

        context.add_parameter_result(parameter_name, parameter_result);
    }

    // Evaluate tests
    let tests = model.get_tests();
    for (test_index, test) in tests {
        let test_result = eval_test(*test_index, test, context);
        context.add_test_result(*test_index, test_result);
    }

    context.pop_active_model(model_path);
}

fn parameter_result_from<E: ExternalEvaluationContext>(
    value: Value,
    expr_span: Span,
    warnings: Vec<EvalWarning>,
    parameter: &ir::Parameter,
    context: &EvalContext<'_, E>,
) -> output::Parameter {
    let (print_level, debug_info) = match parameter.trace_level() {
        ir::TraceLevel::Debug if parameter.is_performance() => {
            let builtin_dependency_values =
                get_builtin_dependency_values(parameter.dependencies().builtin(), context);
            let parameter_dependency_values =
                get_parameter_dependency_values(parameter.dependencies().parameter(), context);
            let external_dependency_values =
                get_external_dependency_values(parameter.dependencies().external(), context);
            (
                output::PrintLevel::Performance,
                Some(output::DebugInfo {
                    builtin_dependency_values,
                    parameter_dependency_values,
                    external_dependency_values,
                }),
            )
        }
        ir::TraceLevel::Trace | ir::TraceLevel::None if parameter.is_performance() => {
            (output::PrintLevel::Performance, None)
        }
        ir::TraceLevel::Debug => {
            let builtin_dependency_values =
                get_builtin_dependency_values(parameter.dependencies().builtin(), context);
            let parameter_dependency_values =
                get_parameter_dependency_values(parameter.dependencies().parameter(), context);
            let external_dependency_values =
                get_external_dependency_values(parameter.dependencies().external(), context);
            (
                output::PrintLevel::Trace,
                Some(output::DebugInfo {
                    builtin_dependency_values,
                    parameter_dependency_values,
                    external_dependency_values,
                }),
            )
        }
        ir::TraceLevel::Trace => (output::PrintLevel::Trace, None),
        ir::TraceLevel::None => (output::PrintLevel::None, None),
    };

    let builtin_dependencies = parameter
        .dependencies()
        .builtin()
        .keys()
        .map(|builtin_name| BuiltinDependency {
            name: builtin_name.clone(),
        })
        .collect::<IndexSet<_>>();

    let parameter_dependencies = parameter
        .dependencies()
        .parameter()
        .keys()
        .map(|parameter_name| ParameterDependency {
            parameter_name: parameter_name.clone(),
        })
        .collect::<IndexSet<_>>();

    let external_dependencies = parameter
        .dependencies()
        .external()
        .iter()
        .map(
            |((reference_name, parameter_name), (model_path, _))| ExternalDependency {
                model_path: model_path.clone(),
                reference_name: reference_name.clone(),
                parameter_name: parameter_name.clone(),
            },
        )
        .collect::<IndexSet<_>>();

    let dependencies = DependencySet {
        builtin_dependencies,
        parameter_dependencies,
        external_dependencies,
    };

    output::Parameter {
        ident: parameter.name().clone(),
        label: parameter.label().clone(),
        value,
        print_level,
        debug_info,
        dependencies,
        expr_span,
        warnings,
    }
}

fn get_evaluation_order(parameters: &IndexMap<ParameterName, ir::Parameter>) -> Vec<ParameterName> {
    let mut evaluation_order = Vec::new();
    let mut visited = HashSet::new();

    for (parameter_name, parameter) in parameters {
        if visited.contains(parameter_name) {
            continue;
        }

        (evaluation_order, visited) = process_parameter_dependencies(
            parameter_name,
            parameter.dependencies(),
            visited,
            evaluation_order,
            parameters,
        );
    }

    evaluation_order
}

fn process_parameter_dependencies(
    parameter_name: &ParameterName,
    parameter_dependencies: &ir::Dependencies,
    mut visited: HashSet<ParameterName>,
    mut evaluation_order: Vec<ParameterName>,
    parameters: &IndexMap<ParameterName, ir::Parameter>,
) -> (Vec<ParameterName>, HashSet<ParameterName>) {
    for dependency in parameter_dependencies.parameter().keys() {
        if visited.contains(dependency) {
            continue;
        }

        let Some(dependency_parameter) = parameters.get(dependency) else {
            // dependency is a builtin value, so we don't need to visit it
            continue;
        };

        (evaluation_order, visited) = process_parameter_dependencies(
            dependency,
            dependency_parameter.dependencies(),
            visited,
            evaluation_order,
            parameters,
        );
    }

    evaluation_order.push(parameter_name.clone());
    visited.insert(parameter_name.clone());

    (evaluation_order, visited)
}

fn eval_test<E: ExternalEvaluationContext>(
    test_index: TestIndex,
    test: &ir::Test,
    context: &mut EvalContext<'_, E>,
) -> Result<output::Test, Vec<EvalError>> {
    context.begin_test_evaluation(test_index);

    let (test_result, expr_span) = eval_expr::eval_expr(test.expr(), context)?;
    let warnings = context.take_expression_warnings();

    match test_result {
        Value::Boolean(true) => Ok(output::Test {
            result: output::TestResult::Passed,
            expr_span: *expr_span,
            warnings,
        }),
        Value::Boolean(false) => {
            let builtin_dependency_values =
                get_builtin_dependency_values(test.dependencies().builtin(), context);
            let parameter_dependency_values =
                get_parameter_dependency_values(test.dependencies().parameter(), context);
            let external_dependency_values =
                get_external_dependency_values(test.dependencies().external(), context);

            let debug_info = Box::new(output::DebugInfo {
                builtin_dependency_values,
                parameter_dependency_values,
                external_dependency_values,
            });
            Ok(output::Test {
                result: output::TestResult::Failed { debug_info },
                expr_span: *expr_span,
                warnings,
            })
        }
        Value::String(_) | Value::Number(_) | Value::MeasuredNumber(_) => {
            Err(vec![EvalError::InvalidType {
                expected_type: ExpectedType::Boolean,
                found_type: test_result.type_(),
                found_span: *expr_span,
            }])
        }
    }
}

/// Gets the values of the builtin dependencies for debug reporting purposes.
fn get_builtin_dependency_values<E: ExternalEvaluationContext>(
    dependencies: &IndexMap<BuiltinValueName, Span>,
    context: &EvalContext<'_, E>,
) -> IndexMap<BuiltinValueName, Value> {
    dependencies
        .keys()
        .map(|dependency| {
            let value = context.lookup_builtin_variable(dependency);
            (dependency.clone(), value)
        })
        .collect::<IndexMap<_, _>>()
}

/// Gets the values of the dependencies for debug reporting purposes.
///
/// This should only be called on expressions that have already been evaluated successfully.
///
/// # Panics
///
/// This function will panic if any of the dependencies are not found.
fn get_parameter_dependency_values<E: ExternalEvaluationContext>(
    dependencies: &IndexMap<ParameterName, Span>,
    context: &EvalContext<'_, E>,
) -> IndexMap<ParameterName, Value> {
    dependencies
        .iter()
        .map(|(dependency, dependency_span)| {
            let value = context
                .lookup_parameter_value(dependency, *dependency_span)
                .expect("dependency should be found because the expression evaluated successfully");

            (dependency.clone(), value)
        })
        .collect::<IndexMap<_, _>>()
}

/// Gets the values of the external dependencies for debug reporting purposes.
///
/// This should only be called on expressions that have already been evaluated successfully.
///
/// # Panics
///
/// This function will panic if any of the dependencies are not found.
fn get_external_dependency_values<E: ExternalEvaluationContext>(
    dependencies: &IndexMap<(ReferenceName, ParameterName), (ModelPath, Span)>,
    context: &EvalContext<'_, E>,
) -> IndexMap<(ReferenceName, ParameterName), Value> {
    dependencies
        .iter()
        .map(
            |((reference_name, parameter_name), (model_path, dependency_span))| {
                let value = context.lookup_model_parameter_value(
                    model_path,
                    parameter_name,
                    *dependency_span,
                );

                let value = value.expect(
                    "dependency should be found because the expression evaluated successfully",
                );

                ((reference_name.clone(), parameter_name.clone()), value)
            },
        )
        .collect::<IndexMap<_, _>>()
}
