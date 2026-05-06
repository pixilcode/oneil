//! Dependency and reference analysis for the runtime.

use oneil_output::DependencySet;
use oneil_shared::{
    paths::ModelPath,
    symbols::{ParameterName, ReferenceName, TestIndex},
};

use crate::{
    context::{ExternalAnalysisContext, TreeContext},
    output::{
        self,
        error::{GetTestValueError, GetValueError, TreeErrors},
    },
};

#[derive(Debug)]
struct TreeValueLocation {
    pub model_path: ModelPath,
    pub reference_name: Option<ReferenceName>,
    pub parameter_name: ParameterName,
}

#[derive(Debug)]
struct GetChildrenResult<T> {
    builtin_children: Vec<output::Tree<T>>,
    parameter_children: Vec<TreeValueLocation>,
    test_children: Vec<output::Tree<T>>,
    test_errors: TreeErrors,
}

/// Gets the dependency tree for a specific parameter.
///
/// The tree shows all parameters, builtin values, and external dependencies
/// that the specified parameter depends on, recursively.
#[must_use]
pub fn get_dependency_tree<E: ExternalAnalysisContext>(
    model_path: &ModelPath,
    parameter_name: &ParameterName,
    external_context: &mut E,
) -> (
    Option<output::Tree<output::DependencyTreeValue>>,
    TreeErrors,
) {
    let location = TreeValueLocation {
        model_path: model_path.clone(),
        reference_name: None,
        parameter_name: parameter_name.clone(),
    };

    get_parameter_tree(
        &location,
        external_context,
        get_dependency_value,
        |location, tree_context| {
            get_dependency_tree_children(
                &location.model_path,
                location.reference_name.as_ref(),
                &location.parameter_name,
                tree_context,
            )
        },
    )
}

fn get_dependency_value<E: ExternalAnalysisContext>(
    location: &TreeValueLocation,
    tree_context: &TreeContext<'_, E>,
) -> Option<Result<output::DependencyTreeValue, GetValueError>> {
    let parameter =
        tree_context.lookup_parameter_value(&location.model_path, &location.parameter_name)?;

    let result = parameter.map(|parameter| {
        let dependency_name = location.reference_name.as_ref().map_or_else(
            || output::DependencyName::Parameter(location.parameter_name.clone()),
            |reference_name| {
                output::DependencyName::External(
                    reference_name.clone(),
                    location.parameter_name.clone(),
                )
            },
        );

        let parameter_value = parameter.value;
        let display_info = Some((location.model_path.clone(), parameter.expr_span));

        output::DependencyTreeValue {
            dependency_name,
            parameter_value,
            display_info,
        }
    });

    Some(result)
}

fn get_dependency_tree_children(
    model_path: &ModelPath,
    reference_name: Option<&ReferenceName>,
    parameter_name: &ParameterName,
    tree_context: &TreeContext<'_, impl ExternalAnalysisContext>,
) -> GetChildrenResult<output::DependencyTreeValue> {
    let DependencySet {
        builtin_dependencies,
        parameter_dependencies,
        external_dependencies,
    } = tree_context.dependents(model_path, parameter_name);

    let builtin_children = builtin_dependencies
        .into_iter()
        .map(|dep| {
            let parameter_value = tree_context
                .lookup_builtin_variable(&dep.name)
                .cloned()
                .expect("the builtin value should be defined");

            let tree_value = output::DependencyTreeValue {
                dependency_name: output::DependencyName::Builtin(dep.name),
                parameter_value,
                display_info: None,
            };

            output::Tree::new(tree_value, Vec::new())
        })
        .collect();

    let parameter_args = parameter_dependencies
        .into_iter()
        .map(|dep| TreeValueLocation {
            model_path: model_path.clone(),
            reference_name: reference_name.cloned(),
            parameter_name: dep.parameter_name,
        });

    let external_args = external_dependencies
        .into_iter()
        .map(|dep| TreeValueLocation {
            model_path: dep.model_path.clone(),
            reference_name: Some(dep.reference_name.clone()),
            parameter_name: dep.parameter_name,
        });

    let parameter_children = parameter_args.chain(external_args).collect();

    GetChildrenResult {
        builtin_children,
        parameter_children,
        test_children: Vec::new(),
        test_errors: TreeErrors::empty(),
    }
}

/// Gets the reference tree for a specific parameter.
///
/// The tree shows all parameters that depend on the specified parameter, recursively.
/// This is the inverse of the dependency tree.
#[must_use]
pub fn get_reference_tree<E: ExternalAnalysisContext>(
    external_context: &mut E,
    model_path: &ModelPath,
    parameter_name: &ParameterName,
) -> (Option<output::Tree<output::ReferenceTreeValue>>, TreeErrors) {
    let location = TreeValueLocation {
        model_path: model_path.clone(),
        reference_name: None,
        parameter_name: parameter_name.clone(),
    };

    get_parameter_tree(
        &location,
        external_context,
        get_reference_value,
        |location, tree_context| {
            get_reference_tree_children(
                &location.model_path,
                &location.parameter_name,
                tree_context,
            )
        },
    )
}

fn get_reference_value<E: ExternalAnalysisContext>(
    location: &TreeValueLocation,
    tree_context: &TreeContext<'_, E>,
) -> Option<Result<output::ReferenceTreeValue, GetValueError>> {
    let parameter =
        tree_context.lookup_parameter_value(&location.model_path, &location.parameter_name)?;

    let result = parameter.map(|parameter| {
        let model_path = location.model_path.clone();
        let parameter_name = location.parameter_name.clone();
        let parameter_value = parameter.value;
        let display_info = (model_path.clone(), parameter.expr_span);

        output::ReferenceTreeValue::Parameter {
            model_path,
            parameter_name,
            parameter_value,
            display_info,
        }
    });

    Some(result)
}

fn get_reference_tree_children(
    model_path: &ModelPath,
    parameter_name: &ParameterName,
    tree_context: &TreeContext<'_, impl ExternalAnalysisContext>,
) -> GetChildrenResult<output::ReferenceTreeValue> {
    enum GetTestError {
        Model(ModelPath),
        Test(ModelPath, TestIndex),
    }

    let deps = tree_context.references(model_path, parameter_name);

    let parameter_children = deps.parameter.into_iter().map(|dep| TreeValueLocation {
        model_path: model_path.clone(),
        reference_name: None,
        parameter_name: dep.parameter_name,
    });

    let external_children = deps
        .external_parameter
        .into_iter()
        .map(|dep| TreeValueLocation {
            model_path: dep.model_path,
            reference_name: None,
            parameter_name: dep.parameter_name,
        });

    let all_param_children = parameter_children.chain(external_children).collect();

    let test_children = deps.test.iter().filter_map(|dep| {
        let test = tree_context.lookup_test_value(model_path, dep.test_index)?;

        let result = test
            .map(|test| {
                let test_passed = test.passed();
                let display_info = (model_path.clone(), test.expr_span);

                output::ReferenceTreeValue::Test {
                    model_path: model_path.clone(),
                    test_index: dep.test_index,
                    test_passed,
                    display_info,
                }
            })
            .map_err(|error| match error {
                GetTestValueError::Model => GetTestError::Model(model_path.clone()),
                GetTestValueError::Test => GetTestError::Test(model_path.clone(), dep.test_index),
            });

        Some(result)
    });

    let test_external_children = deps.external_test.iter().filter_map(|dep| {
        let test = tree_context.lookup_test_value(&dep.model_path, dep.test_index)?;

        let result = test
            .map(|test| {
                let test_passed = test.passed();
                let display_info = (dep.model_path.clone(), test.expr_span);

                output::ReferenceTreeValue::Test {
                    model_path: dep.model_path.clone(),
                    test_index: dep.test_index,
                    test_passed,
                    display_info,
                }
            })
            .map_err(|error| match error {
                GetTestValueError::Model => GetTestError::Model(dep.model_path.clone()),
                GetTestValueError::Test => {
                    GetTestError::Test(dep.model_path.clone(), dep.test_index)
                }
            });

        Some(result)
    });

    let (all_test_children, test_errors) = test_children.chain(test_external_children).fold(
        (Vec::new(), TreeErrors::empty()),
        |(mut children, mut errors), result| {
            match result {
                Ok(child) => {
                    children.push(output::Tree::new(child, Vec::new()));
                }
                Err(GetTestError::Model(model_path)) => {
                    errors.insert_model_error(model_path);
                }
                Err(GetTestError::Test(model_path, test_index)) => {
                    errors.insert_test_error(model_path, test_index);
                }
            }

            (children, errors)
        },
    );

    GetChildrenResult {
        // no builtins reference other parameters
        builtin_children: Vec::new(),
        parameter_children: all_param_children,
        test_children: all_test_children,
        test_errors,
    }
}

/// Unified implementation for dependency and reference trees.
///
/// Recursively builds a tree of parameter values, using `get_value` to resolve
/// each node and `get_children` to determine the values for the children.
fn get_parameter_tree<V: std::fmt::Debug, E: ExternalAnalysisContext, GetVal, GetChildren>(
    location: &TreeValueLocation,
    external_context: &mut E,
    get_value: GetVal,
    get_children: GetChildren,
) -> (Option<output::Tree<V>>, TreeErrors)
where
    GetVal: Fn(&TreeValueLocation, &TreeContext<'_, E>) -> Option<Result<V, GetValueError>>,
    GetChildren: Fn(&TreeValueLocation, &TreeContext<'_, E>) -> GetChildrenResult<V>,
{
    let dependency_graph = get_dependency_graph(external_context);

    let tree_context = TreeContext::new(external_context, dependency_graph);

    return recurse(location, &tree_context, &get_value, &get_children);

    #[expect(
        clippy::items_after_statements,
        reason = "this is an internal recursive function, we keep it here for clarity"
    )]
    fn recurse<V: std::fmt::Debug, E: ExternalAnalysisContext, GetVal, GetChildren>(
        location: &TreeValueLocation,
        tree_context: &TreeContext<'_, E>,
        get_value: &GetVal,
        get_children: &GetChildren,
    ) -> (Option<output::Tree<V>>, TreeErrors)
    where
        GetVal: Fn(&TreeValueLocation, &TreeContext<'_, E>) -> Option<Result<V, GetValueError>>,
        GetChildren: Fn(&TreeValueLocation, &TreeContext<'_, E>) -> GetChildrenResult<V>,
    {
        // get the value for the current location
        let Some(value) = get_value(location, tree_context) else {
            // if it doesn't exist, return no tree and no errors
            return (None, TreeErrors::empty());
        };

        let value = match value {
            Ok(value) => value,
            Err(GetValueError::Model) => {
                let mut tree_errors = TreeErrors::empty();
                tree_errors.insert_model_error(location.model_path.clone());

                return (None, tree_errors);
            }
            Err(GetValueError::Parameter) => {
                let mut tree_errors = TreeErrors::empty();
                tree_errors.insert_parameter_error(
                    location.model_path.clone(),
                    location.parameter_name.clone(),
                );

                return (None, tree_errors);
            }
        };

        // get the children for the current location
        let GetChildrenResult {
            builtin_children,
            parameter_children,
            test_children,
            test_errors,
        } = get_children(location, tree_context);

        // recurse on the parameter children
        let (parameter_children, mut tree_errors) = parameter_children
            .into_iter()
            .map(|location| recurse(&location, tree_context, get_value, get_children))
            .fold(
                (Vec::new(), TreeErrors::empty()),
                |(mut children, mut errors), (child, child_errors)| {
                    children.extend(child);
                    errors.extend(child_errors);
                    (children, errors)
                },
            );

        let children = builtin_children
            .into_iter()
            .chain(parameter_children)
            .chain(test_children)
            .collect();

        tree_errors.extend(test_errors);

        (Some(output::Tree::new(value, children)), tree_errors)
    }
}

/// Gets the dependency graph for all models in the evaluation cache.
///
/// The graph is built from the cached evaluation results. The cache must
/// have been populated by a prior call to [`Runtime::load_ir`]. This
/// can be done indirectly by calling [`Runtime::eval_model`].
#[must_use]
fn get_dependency_graph<E: ExternalAnalysisContext>(
    external_context: &E,
) -> crate::dep_graph::DependencyGraph {
    let mut dependency_graph = crate::dep_graph::DependencyGraph::new();

    for (model_path, model) in external_context.get_all_model_ir() {
        // Resolve the model path for an external `parameter.reference`
        // dependency. The path is no longer stored in `Dependencies`
        // (it is resolved lazily from the live instance graph), so we
        // look it up from the model's own reference/submodel maps here.
        let resolve_external_path = |reference_name: &oneil_shared::symbols::ReferenceName| {
            model
                .references()
                .get(reference_name)
                .map(|r| &r.path)
                .or_else(|| {
                    model
                        .submodels()
                        .get(reference_name)
                        .map(|s| s.instance.path())
                })
        };

        for (parameter_name, parameter) in model.parameters() {
            let dependencies = parameter.dependencies();

            for builtin_dep in dependencies.builtin().keys() {
                dependency_graph.add_depends_on_builtin(
                    model_path.clone(),
                    parameter_name.clone(),
                    oneil_output::BuiltinDependency {
                        name: builtin_dep.clone(),
                    },
                );
            }

            for parameter_dep in dependencies.parameter().keys() {
                dependency_graph.add_depends_on_parameter(
                    model_path.clone(),
                    parameter_name.clone(),
                    oneil_output::ParameterDependency {
                        parameter_name: parameter_dep.clone(),
                    },
                );
            }

            for ((reference_dep_name, parameter_dep_name), _span) in dependencies.external() {
                let Some(external_model_path) = resolve_external_path(reference_dep_name) else {
                    continue;
                };
                dependency_graph.add_depends_on_external(
                    model_path.clone(),
                    parameter_name.clone(),
                    oneil_output::ExternalDependency {
                        model_path: external_model_path.clone(),
                        reference_name: reference_dep_name.clone(),
                        parameter_name: parameter_dep_name.clone(),
                    },
                );
            }
        }

        for (test_index, test) in model.tests() {
            let dependencies = test.dependencies();

            for parameter_dep in dependencies.parameter().keys() {
                dependency_graph.add_test_depends_on_parameter(
                    model_path.clone(),
                    *test_index,
                    oneil_output::ParameterDependency {
                        parameter_name: parameter_dep.clone(),
                    },
                );
            }

            for ((reference_dep_name, parameter_dep_name), _span) in dependencies.external() {
                let Some(external_model_path) = resolve_external_path(reference_dep_name) else {
                    continue;
                };
                dependency_graph.add_test_depends_on_external(
                    model_path.clone(),
                    *test_index,
                    oneil_output::ExternalDependency {
                        model_path: external_model_path.clone(),
                        reference_name: reference_dep_name.clone(),
                        parameter_name: parameter_dep_name.clone(),
                    },
                );
            }
        }
    }

    dependency_graph
}
