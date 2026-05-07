//! Resolver pass: file-static lowering from AST to
//! [`InstancedModel`](crate::InstancedModel) templates.
//!
//! Each `.on` / `.one` file is parsed and lowered into a template
//! together with its declarative design metadata; the templates are
//! then handed to the instancing pass ([`crate::instance`]) for
//! per-unit graph build and composition.
//!
//! Responsibilities:
//!
//! - Model parsing and AST processing
//! - Python import validation
//! - Submodel / reference resolution
//! - Design surface resolution (`design <target>`, `apply X to ref`,
//!   design-local parameters and overlays)
//! - Parameter and test expression resolution
//!
//! Cross-file cycles are *not* the resolver's concern: the
//! active-model set here is purely a recursion guard, while the
//! per-unit instance-graph build owns cycle diagnostics (see
//! [`CompilationCycleError`](crate::CompilationCycleError) and
//! `docs/decisions/2026-04-24-two-pass-instance-graph.md`).
//!

use oneil_ast as ast;
use oneil_ir as ir;
use oneil_shared::paths::ModelPath;

use crate::{ExternalResolutionContext, ResolutionContext};

mod resolve_design;
pub use resolve_design::collect_design_target_path;
mod resolve_expr;
mod resolve_model_import;
mod resolve_parameter;
mod resolve_python_import;
mod resolve_test;
mod resolve_trace_level;
mod resolve_unit;
mod resolve_variable;
mod util;

pub use resolve_expr::resolve_expr;
pub use util::{ParameterWithSection, TestWithSection};

/// Loads a model and all its dependencies, building a complete model collection.
pub fn load_model<E>(model_path: &ModelPath, resolution_context: &mut ResolutionContext<'_, E>)
where
    E: ExternalResolutionContext,
{
    // Break the recursion when this model is already being loaded.
    //
    // Cycle *diagnostics* are now emitted by the per-unit instance-graph
    // build (see `oneil_frontend::instance::graph::build_unit_graph_uncached`),
    // which has access to the back-edge reference span and the full
    // `CompilationUnit` chain. Emitting one here too would (a) duplicate
    // the diagnostic and (b) flag the partially-loaded model as
    // erroring, which prevents the parent's `resolve_model_imports`
    // from registering the cyclic reference at all — and without that
    // reference the graph-build can't see the cycle either. So this
    // path is purely a recursion guard.
    if resolution_context.is_model_active(model_path) {
        return;
    }

    // check if model is already been visited, then mark as visited if not
    if resolution_context.has_visited_model(model_path) {
        return;
    }

    // push the model onto the active models stack
    resolution_context.push_active_model(model_path);

    // parse model ast
    let load_ast_result = resolution_context.load_ast(model_path);

    // get the model ast
    //
    // note that this succeeds even if the model ast is only partially loaded
    let Some(model_ast) = load_ast_result.value() else {
        resolution_context.record_failed_ast_load(model_path.clone());
        resolution_context.pop_active_model(model_path);
        return;
    };
    let model_ast = (*model_ast).clone();

    let model_note = model_ast
        .note()
        .map(|n| ir::Note::new(n.value().to_string()));
    resolution_context.set_active_model_note(model_note);

    // split model ast into imports, use models, parameters, and tests
    let (imports, model_imports, parameters, tests) = split_model_ast(&model_ast);

    // resolve python imports
    resolve_python_import::resolve_python_imports(model_path, imports, resolution_context);

    // load the models imported
    load_model_imports(model_path, &model_imports, resolution_context);

    resolve_design::preload_design_files(model_path, &model_ast, resolution_context);

    // resolve submodels and references to external models
    resolve_model_import::resolve_model_imports(model_path, model_imports, resolution_context);

    // resolve the design surface (`design <target>`, `apply X to ref`, design params/overlays).
    // This runs BEFORE parameters so that the consuming model knows which references
    // will receive design-applied parameters by the time `param.ref` lookups are processed.
    resolve_design::resolve_design_surface(model_path, &model_ast, resolution_context);

    // resolve parameters
    resolve_parameter::resolve_parameters(parameters, resolution_context);

    // resolve tests
    resolve_test::resolve_tests(tests, resolution_context);

    // pop the model from the active models stack
    resolution_context.pop_active_model(model_path);
}

/// Splits a model AST into its constituent declaration types.
///
/// This function processes the declarations in a model AST and categorizes them into
/// separate collections for imports, use models, parameters, and tests. It processes
/// both top-level declarations and declarations within sections.
///
/// # Arguments
///
/// * `model_ast` - The parsed model AST containing all declarations
///
/// # Returns
///
/// A tuple containing:
/// * `Vec<&ImportNode>` - All import declarations from the model
/// * `Vec<&SubmodelDeclNode>` - All submodel declarations from the model
/// * `Vec<ParameterWithSection>` - Parameter declarations with optional enclosing section
/// * `Vec<TestWithSection>` - Test declarations with optional enclosing section
///
/// # Behavior
///
/// The function processes declarations in the following order:
/// 1. Top-level declarations in the model
/// 2. Declarations within each section of the model
///
/// This separation is necessary for the different processing steps in model loading.
fn split_model_ast(
    model_ast: &ast::Model,
) -> (
    Vec<&ast::ImportNode>,
    Vec<&ast::SubmodelDeclNode>,
    Vec<ParameterWithSection<'_>>,
    Vec<TestWithSection<'_>>,
) {
    let mut imports = vec![];
    let mut submodels = vec![];
    let mut parameters = vec![];
    let mut tests = vec![];

    let top_level = model_ast.decls().iter().map(|d| (d, None));
    let in_sections = model_ast
        .sections()
        .iter()
        .flat_map(|s| s.decls().iter().map(move |d| (d, Some(s.header().label()))));

    for (decl, section_label) in top_level.chain(in_sections) {
        match &**decl {
            ast::Decl::Import(import) => imports.push(import),
            ast::Decl::Submodel(submodel) => {
                submodels.push(submodel);
            }
            ast::Decl::Parameter(parameter) => parameters.push(ParameterWithSection {
                parameter,
                section_label,
            }),
            ast::Decl::Test(test) => tests.push(TestWithSection {
                test,
                section_label,
            }),
            // Design declarations are handled by resolve_design_surface.
            ast::Decl::DesignTarget(_)
            | ast::Decl::ApplyDesign(_)
            | ast::Decl::DesignParameter(_) => {}
        }
    }

    (imports, submodels, parameters, tests)
}

/// Recursively loads all submodels referenced by a model.
///
/// This function processes all submodel declarations in a model and recursively
/// loads each referenced model. It maintains the loading stack for circular
/// dependency detection and accumulates all loaded models in the builder.
///
/// # Arguments
///
/// * `model_path` - The path of the current model (used for resolving relative paths)
/// * `load_stack` - The loading stack for circular dependency detection
/// * `file_loader` - The file loader for parsing referenced models
/// * `model_imports` - The submodel declarations to process
/// * `builder` - The model collection builder to accumulate results
///
/// # Returns
///
/// Returns the updated model collection builder containing all loaded submodels.
///
/// # Circular Dependency Handling
///
/// The function pushes the current model path onto the load stack before processing
/// submodels and pops it after processing is complete. This ensures that circular
/// dependencies are properly detected during the recursive loading process.
///
/// # Path Resolution
///
/// Submodel paths are resolved relative to the current model path using
/// `model_path.get_sibling_path(&submodel.model_name)`.
fn load_model_imports<E>(
    model_path: &ModelPath,
    model_imports: &[&ast::SubmodelDeclNode],
    resolution_context: &mut ResolutionContext<'_, E>,
) where
    E: ExternalResolutionContext,
{
    for model_import in model_imports {
        // get the imported model path
        let model_import_relative_path = model_import.get_model_relative_path();
        let model_import_path = model_path.get_sibling_model_path(model_import_relative_path);

        // load the imported model (and its submodels)
        load_model(&model_import_path, resolution_context);

        // If the .on file didn't exist, check for a sibling .one design file.
        // Design files carry their own target model declaration (`design <name>`),
        // so loading the .one triggers loading of that target model as well.
        if model_import.model_info().subcomponents().is_empty()
            && resolution_context.ast_load_failed(&model_import_path)
        {
            let design_relative = model_import.get_design_relative_path();
            let design_path = model_path.get_sibling_design_path(design_relative);
            load_model(&design_path.to_model_path(), resolution_context);
        }
    }
}

#[cfg(test)]
mod tests {
    use oneil_ast as ast;
    use oneil_ir as ir;
    use oneil_shared::symbols::{ParameterName, ReferenceName};

    use super::*;
    use crate::test::{external_context::TestExternalContext, test_ast, test_model_path};

    #[test]
    fn split_model_ast_empty() {
        let _model_path = test_model_path("test");
        let model = test_ast::empty_model_node();
        let (imports, use_models, parameters, tests) = split_model_ast(&model);

        assert!(imports.is_empty());
        assert!(use_models.is_empty());
        assert!(parameters.is_empty());
        assert!(tests.is_empty());
    }

    #[test]
    fn split_model_ast_with_all_declarations() {
        let _model_path = test_model_path("test");
        let model = test_ast::ModelBuilder::new()
            .with_submodel("submodel")
            .build();
        let (imports, use_models, parameters, tests) = split_model_ast(&model);

        assert_eq!(imports.len(), 0);
        assert_eq!(use_models.len(), 1);
        assert_eq!(
            use_models[0].model_info().top_component().as_str(),
            "submodel"
        );
        assert!(use_models[0].model_info().subcomponents().is_empty());
        assert!(parameters.is_empty());
        assert!(tests.is_empty());
    }

    #[test]
    fn split_model_ast_use_model_only() {
        let _model_path = test_model_path("test");
        let model = test_ast::ModelBuilder::new()
            .with_submodel("submodel1")
            .with_submodel("submodel2")
            .build();
        let (imports, use_models, parameters, tests) = split_model_ast(&model);

        assert!(imports.is_empty());
        assert_eq!(use_models.len(), 2);
        assert_eq!(
            use_models[0].model_info().top_component().as_str(),
            "submodel1"
        );
        assert_eq!(
            use_models[1].model_info().top_component().as_str(),
            "submodel2"
        );

        assert!(parameters.is_empty());
        assert!(tests.is_empty());
    }

    #[test]
    fn load_model_success() {
        let model_path = test_model_path("test");
        let mut external =
            TestExternalContext::new().with_model_asts([("test.on", test_ast::empty_model_node())]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&model_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 1);
        assert!(results.contains_key(&model_path));

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_model_parse_error() {
        // Path "nonexistent" has no AST in context -> load_ast fails
        let model_path = test_model_path("nonexistent");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&model_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the model errors (parse failure does not record an error in model_errors)
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_model_circular_dependency() {
        // Cycle `main.on -> sub.on -> main.on`. The resolver's role is
        // just to load both files and record their submodel/reference
        // chains; the dedicated cycle diagnostic comes from the
        // per-unit instance-graph build (see
        // `oneil_frontend::instance::graph::build_unit_graph_uncached`),
        // not from `load_model`.
        let main_path = test_model_path("main");
        let sub_path = test_model_path("sub");
        let main_test_model = test_ast::ModelBuilder::new().with_submodel("sub").build();
        let sub_test_model = test_ast::ModelBuilder::new().with_submodel("main").build();
        let mut external = TestExternalContext::new().with_model_asts([
            ("main.on", test_ast::model_node(main_test_model)),
            ("sub.on", test_ast::model_node(sub_test_model)),
        ]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&main_path, &mut resolution_context);

        let results = resolution_context.into_result();

        assert_eq!(results.len(), 2);
        assert!(results.contains_key(&main_path));
        assert!(results.contains_key(&sub_path));

        // Both files load cleanly at the resolver layer: each registers
        // its submodel/reference chain pointing at the other, with no
        // resolver-level errors. The graph build sees both edges and
        // emits the dedicated cycle diagnostic against each
        // participating file.
        for (path, result) in &results {
            assert!(
                result.model_errors().is_empty(),
                "expected no resolver-level errors on {path:?}",
            );
        }

        // `submodel` declarations populate `submodels()` only — the
        // `references()` map is reserved for `reference` imports.
        let main_subs = results
            .get(&main_path)
            .expect("main result")
            .model()
            .submodels();
        assert_eq!(main_subs.len(), 1);
        assert_eq!(
            main_subs[&ReferenceName::from("sub")].instance.path(),
            &sub_path
        );

        let sub_subs = results
            .get(&sub_path)
            .expect("sub result")
            .model()
            .submodels();
        assert_eq!(sub_subs.len(), 1);
        assert_eq!(
            sub_subs[&ReferenceName::from("main")].instance.path(),
            &main_path
        );
    }

    #[test]
    fn load_model_already_visited() {
        // Load the same model twice
        let model_path = test_model_path("test");
        let mut external =
            TestExternalContext::new().with_model_asts([("test.on", test_ast::empty_model_node())]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&model_path, &mut resolution_context);
        load_model(&model_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 1);
        assert!(results.contains_key(&model_path));

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_use_models_empty() {
        // Load a model with no use/ref declarations (only parent in context).
        let model_path = test_model_path("parent");
        let mut external = TestExternalContext::new()
            .with_model_asts([("parent.on", test_ast::empty_model_node())]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&model_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 1);
        assert!(results.contains_key(&model_path));

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_use_models_with_existing_models() {
        // Load parent that uses child1 and child2; all three ASTs in context.
        let parent_path = test_model_path("parent");
        let parent_ast = test_ast::ModelBuilder::new()
            .with_submodel("child1")
            .with_submodel("child2")
            .build();
        let mut external = TestExternalContext::new().with_model_asts([
            ("parent.on", test_ast::model_node(parent_ast)),
            ("child1.on", test_ast::empty_model_node()),
            ("child2.on", test_ast::empty_model_node()),
        ]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&parent_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 3);
        assert!(results.contains_key(&parent_path));
        assert!(results.contains_key(&test_model_path("child1")));
        assert!(results.contains_key(&test_model_path("child2")));

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_use_models_with_parse_errors() {
        // Parent uses "nonexistent"; that path has no AST in context.
        let parent_path = test_model_path("parent");
        let parent_ast = test_ast::ModelBuilder::new()
            .with_submodel("nonexistent")
            .build();
        let mut external = TestExternalContext::new()
            .with_model_asts([("parent.on", test_ast::model_node(parent_ast))]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&parent_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the model errors
        assert!(results.contains_key(&parent_path));
    }

    #[test]
    fn load_model_complex_dependency_chain() {
        // Dependency chain: root.on -> level1.on -> level2.on
        let root_path = test_model_path("root");
        let root_model = test_ast::ModelBuilder::new()
            .with_submodel("level1")
            .build();
        let level1_model = test_ast::ModelBuilder::new()
            .with_submodel("level2")
            .build();
        let level2_model = test_ast::empty_model();

        let mut external = TestExternalContext::new().with_model_asts([
            ("root.on", test_ast::model_node(root_model)),
            ("level1.on", test_ast::model_node(level1_model)),
            ("level2.on", test_ast::model_node(level2_model)),
        ]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&root_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 3);
        assert!(results.contains_key(&root_path));
        assert!(results.contains_key(&test_model_path("level1")));
        assert!(results.contains_key(&test_model_path("level2")));

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_model_with_sections() {
        // Model with a section that declares a use submodel
        let test_path = test_model_path("test");
        let submodel_node = test_ast::empty_model();
        let use_model_decl = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("submodel")
            .with_kind(ast::ModelKind::Submodel)
            .build_as_decl_node();
        let model_node = test_ast::ModelBuilder::new()
            .with_section("section1", vec![use_model_decl])
            .build();

        let mut external = TestExternalContext::new().with_model_asts([
            ("test.on", test_ast::model_node(model_node)),
            ("submodel.on", test_ast::model_node(submodel_node)),
        ]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&test_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 2);
        assert!(results.contains_key(&test_path));
        assert!(results.contains_key(&test_model_path("submodel")));

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    #[test]
    fn load_model_with_reference() {
        // Main model has ref "reference" and parameter y = reference.x
        let test_path = test_model_path("test");
        let reference_node = test_ast::ModelBuilder::new()
            .with_number_parameter("x", 1.0)
            .build();
        let model_node = test_ast::ModelBuilder::new()
            .with_reference("reference")
            .with_reference_variable_parameter("y", "reference", "x")
            .build();

        let mut external = TestExternalContext::new().with_model_asts([
            ("test.on", test_ast::model_node(model_node)),
            ("reference.on", test_ast::model_node(reference_node)),
        ]);
        let mut resolution_context = ResolutionContext::new(&mut external);

        load_model(&test_path, &mut resolution_context);

        let results = resolution_context.into_result();

        // check the models generated
        assert_eq!(results.len(), 2);
        let main_model = results
            .get(&test_path)
            .expect("main model should be present")
            .model();
        let y_parameter = main_model
            .get_parameter(&ParameterName::from("y"))
            .expect("y parameter should be present");

        let ir::ParameterValue::Simple(y_parameter_value, _) = y_parameter.value() else {
            panic!("y parameter value should be a simple value");
        };

        let ir::Expr::Variable {
            span: _,
            variable: variable_expr,
        } = &**y_parameter_value
        else {
            panic!("y parameter value should be a variable expression");
        };

        let ir::Variable::External {
            reference_name,
            parameter_name,
            ..
        } = variable_expr
        else {
            panic!("variable expression should be an external variable");
        };

        assert_eq!(reference_name.as_str(), "reference");
        assert_eq!(parameter_name.as_str(), "x");

        // check the model errors
        assert!(results.values().all(|r| r.model_errors().is_empty()));
    }

    // Removed: `load_model_with_submodel_with_error` previously relied on
    // the file-time existence check inside `resolve_variable.rs` to surface
    // an "undefined parameter" error when a referenced model was missing
    // from the external context. That check has been deferred to the
    // post-build validation pass in
    // [`oneil_analysis::validate_instance_graph`], so the file resolver
    // no longer treats missing-reference-AST as a model error
    // (consistent with `load_use_models_with_parse_errors`).
}
