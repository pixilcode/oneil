use indexmap::IndexMap;
use oneil_ir as ir;
use oneil_shared::{EvalInstanceKey, partial::MaybePartialResult, paths::ModelPath};

use oneil_output::{self as output, Model, Value};

use crate::{
    EvalError,
    context::{EvalContext, ExternalEvaluationContext},
    error::{EvalErrors, ExpectedType},
    eval_expr, eval_parameter,
    instance_graph::InstanceGraph,
};

/// Evaluates the model at `model_path` and every instance reachable from it.
///
/// The function builds an [`InstanceGraph`] from the IR (no runtime-supplied designs),
/// seeds an [`EvalContext`] from the graph, then drives two passes:
///
/// 1. **Force** ([`force_all_models`]): for each instance, force every still-pending
///    parameter (the lazy memo table handles cross-model demand on the fly) and
///    evaluate the instance's tests.
/// 2. **Error propagation** ([`propagate_reference_errors`]): record on each parent
///    instance any references whose evaluation produced errors so that callers see
///    transitive failures without searching the tree themselves.
///
/// All structural work — reference replacements, design overlays, design-introduced
/// parameters, extracted submodel wiring — is performed by [`InstanceGraph::build`]
/// before evaluation begins.
pub fn eval_model<E: ExternalEvaluationContext>(
    model_path: &ModelPath,
    external_context: &mut E,
) -> IndexMap<EvalInstanceKey, MaybePartialResult<Model, EvalErrors>> {
    eval_model_with_designs(model_path, &[], external_context)
}

/// Like [`eval_model`], but also applies the given runtime-supplied
/// [`ir::DesignApplication`]s at the root before evaluation.
///
/// These are typically the contributions implied by a CLI `--design <file>` flag.
pub fn eval_model_with_designs<E: ExternalEvaluationContext>(
    model_path: &ModelPath,
    runtime_designs: &[ir::DesignApplication],
    external_context: &mut E,
) -> IndexMap<EvalInstanceKey, MaybePartialResult<Model, EvalErrors>> {
    let graph = InstanceGraph::build(model_path, runtime_designs, external_context);
    eval_model_from_graph(&graph, external_context)
}

/// Evaluates every instance in `graph`, returning per-instance results.
///
/// Use this entry point when callers want to supply a graph built externally — for
/// example with runtime-supplied design applications (CLI `--design`).
pub fn eval_model_from_graph<E: ExternalEvaluationContext>(
    graph: &InstanceGraph,
    external_context: &mut E,
) -> IndexMap<EvalInstanceKey, MaybePartialResult<Model, EvalErrors>> {
    let mut context = EvalContext::from_graph(graph, external_context);

    force_all_models(graph, &mut context);
    propagate_reference_errors(&mut context);

    context.into_result()
}

/// Drives lazy forcing of every pending parameter on every instance and evaluates tests.
///
/// Iteration order over instances doesn't matter: cross-model dependencies are
/// resolved through [`EvalContext::lookup_external_parameter_value`]'s lazy memo, which
/// itself manages per-evaluation scope push/pop.
fn force_all_models<E: ExternalEvaluationContext>(
    graph: &InstanceGraph,
    context: &mut EvalContext<'_, E>,
) {
    let keys: Vec<EvalInstanceKey> = context.model_keys_snapshot();

    for key in keys {
        context.force_all_pending_on(&key);

        // Tests are read from the graph's instance entry. They need a current scope for
        // unprefixed parameter lookups inside test expressions.
        let Some(instanced) = graph.instances.get(&key) else {
            continue;
        };
        if instanced.tests.is_empty() {
            continue;
        }
        context.push_active_model(key.clone());
        let test_pairs: Vec<(_, _)> = instanced
            .tests
            .iter()
            .map(|(idx, test)| (*idx, test.clone()))
            .collect();
        for (test_index, test) in test_pairs {
            let test_result = eval_test(&test, context);
            context.add_test_result(&key, test_index, test_result);
        }
        context.pop_active_model(&key);
    }
}

/// After forcing, every parent instance inspects its registered references for errors
/// and records them on itself so downstream consumers can see transitive failure.
fn propagate_reference_errors<E: ExternalEvaluationContext>(context: &mut EvalContext<'_, E>) {
    let pairs: Vec<(EvalInstanceKey, EvalInstanceKey)> = context.reference_pairs_snapshot();
    for (parent_key, child_key) in pairs {
        if context.reference_has_errors(&child_key) {
            context.add_reference_error_to(&parent_key, &child_key);
        }
    }
}

/// Evaluates a single test in the context of the currently active scope.
fn eval_test<E: ExternalEvaluationContext>(
    test: &ir::Test,
    context: &mut EvalContext<'_, E>,
) -> Result<output::Test, Vec<EvalError>> {
    let (test_result, expr_span) = eval_expr::eval_expr(test.expr(), context)?;
    let expr_span = *expr_span;

    match test_result {
        Value::Boolean(true) => Ok(output::Test {
            result: output::TestResult::Passed,
            expr_span,
        }),
        Value::Boolean(false) => {
            let builtin_dependency_values = eval_parameter::get_builtin_dependency_values(
                test.dependencies().builtin(),
                context,
            );
            let parameter_dependency_values = eval_parameter::get_parameter_dependency_values(
                test.dependencies().parameter(),
                context,
            );
            let external_dependency_values = eval_parameter::get_external_dependency_values(
                test.dependencies().external(),
                context,
            );

            let debug_info = Box::new(output::DebugInfo {
                builtin_dependency_values,
                parameter_dependency_values,
                external_dependency_values,
            });
            Ok(output::Test {
                result: output::TestResult::Failed { debug_info },
                expr_span,
            })
        }
        Value::String(_) | Value::Number(_) | Value::MeasuredNumber(_) => {
            Err(vec![EvalError::InvalidType {
                expected_type: ExpectedType::Boolean,
                found_type: test_result.type_(),
                found_span: expr_span,
            }])
        }
    }
}
