//! Submodel resolution for the Oneil model loader

use oneil_ast as ast;
use oneil_ir as ir;
use oneil_shared::{
    paths::ModelPath,
    search::search,
    span::Span,
    symbols::{ReferenceName, SubmodelName},
};

use crate::{
    ExternalResolutionContext, ResolutionContext,
    context::{MAX_BEST_MATCH_DISTANCE, ModelResult},
    error::ModelImportResolutionError,
};

/// Resolves submodels and their associated tests from use model declarations.
pub fn resolve_model_imports<E>(
    model_path: &ModelPath,
    model_imports: Vec<&ast::UseModelNode>,
    resolution_context: &mut ResolutionContext<'_, E>,
) where
    E: ExternalResolutionContext,
{
    for model_import in model_imports {
        let import_path = calc_import_path(model_path, model_import);

        let (reference_name, reference_name_span) =
            get_reference_name_and_span(model_import.model_info());
        // The "source name" carried on the `SubmodelImport` for diagnostics
        // is the model file name as written (`foo` in `use foo as bar`).
        let (source_name, source_name_span) =
            get_source_name_and_span(model_import.model_info());

        let is_submodel = model_import.model_kind() == ast::ModelKind::Submodel;

        // check for duplicates — both maps key by the alias (reference name).
        let maybe_reference_duplicate_error = resolution_context
            .get_reference_from_active_model(&reference_name)
            .map(|original_reference| {
                ModelImportResolutionError::duplicate_reference(
                    reference_name.clone(),
                    *original_reference.name_span(),
                    reference_name_span,
                )
            });

        let maybe_submodel_duplicate_error = is_submodel
            .then(|| {
                resolution_context
                    .get_submodel_from_active_model(&reference_name)
                    .map(|original_submodel| {
                        ModelImportResolutionError::duplicate_submodel(
                            SubmodelName::from(reference_name.as_str()),
                            *original_submodel.name_span(),
                            reference_name_span,
                        )
                    })
            })
            .flatten();

        let had_duplicate = maybe_reference_duplicate_error.is_some()
            || maybe_submodel_duplicate_error.is_some();

        // handle duplicate references
        if let Some(reference_duplicate_error) = maybe_reference_duplicate_error {
            let submodel_name =
                is_submodel.then(|| SubmodelName::from(reference_name.as_str()));
            resolution_context.add_model_import_resolution_error_to_active_model(
                reference_name.clone(),
                submodel_name,
                reference_duplicate_error,
            );
        }

        // handle duplicate submodels if the use model is a submodel
        if let Some(submodel_duplicate_error) = maybe_submodel_duplicate_error {
            resolution_context.add_model_import_resolution_error_to_active_model(
                reference_name.clone(),
                Some(SubmodelName::from(reference_name.as_str())),
                submodel_duplicate_error,
            );
        }

        // if there were any duplicates, stop processing this use model
        if had_duplicate {
            continue;
        }

        // resolve the path for the use model
        let subcomponents = model_import.model_info().subcomponents();
        let resolved_path = resolve_model_path(
            import_path,
            reference_name_span,
            subcomponents,
            resolution_context,
        );

        // handle the error if there was one
        let resolved_path = match resolved_path {
            Ok(resolved_path) => resolved_path,
            Err(error) => {
                // Errors continue to surface the alias under the
                // `Option<SubmodelName>` slot for stability with existing
                // diagnostics — the alias is what the user typed at this
                // declaration site.
                handle_resolution_error(
                    *error,
                    model_import,
                    reference_name.clone(),
                    SubmodelName::from(reference_name.as_str()),
                    reference_name_span,
                    is_submodel,
                    resolution_context,
                );

                continue;
            }
        };

        // add the submodel to the active model if it's a submodel — keyed by alias.
        if is_submodel {
            resolution_context.add_submodel_to_active_model(
                reference_name.clone(),
                source_name,
                source_name_span,
            );
        }

        // add the reference to the active model
        resolution_context.add_reference_to_active_model(
            reference_name.clone(),
            reference_name_span,
            resolved_path.clone(),
        );

        let Some(submodel_list) = model_import.imported_submodels() else {
            // if we don't have any imported submodels, we're done
            continue;
        };

        resolve_extracted_submodels(
            &resolved_path,
            &reference_name,
            submodel_list,
            resolution_context,
        );
    }
}

/// Resolves extracted submodels from a `with` clause.
///
/// Creates both:
/// 1. `SubmodelImport` entries that store the relative path within the parent
///    (for eval-time re-resolution through potentially replaced parent)
/// 2. `ReferenceImport` entries with the resolved path (for variable validation)
fn resolve_extracted_submodels<E>(
    parent_model_path: &ModelPath,
    parent_reference_name: &ReferenceName,
    submodel_list: &oneil_ast::Node<oneil_ast::SubmodelList>,
    resolution_context: &mut ResolutionContext<'_, E>,
) where
    E: ExternalResolutionContext,
{
    for submodel_info in submodel_list.iter() {
        // get the subcomponents relative to the main model being imported
        let mut submodel_subcomponents = submodel_info.subcomponents().to_vec();
        submodel_subcomponents.insert(0, submodel_info.top_component().clone());

        // get the reference name (alias) for the extracted submodel
        let (reference_name, reference_name_span) = get_reference_name_and_span(submodel_info);
        let (source_name, source_name_span) = get_source_name_and_span(submodel_info);

        // check for duplicate references
        let maybe_original_reference =
            resolution_context.get_reference_from_active_model(&reference_name);
        if let Some(original_reference) = maybe_original_reference {
            let error = ModelImportResolutionError::duplicate_reference(
                reference_name.clone(),
                *original_reference.name_span(),
                reference_name_span,
            );

            resolution_context.add_model_import_resolution_error_to_active_model(
                reference_name.clone(),
                None,
                error,
            );

            continue;
        }

        // validate the submodel path exists (type-checking)
        let resolved_reference_path = resolve_model_path(
            parent_model_path.clone(),
            reference_name_span,
            &submodel_subcomponents,
            resolution_context,
        );

        match resolved_reference_path {
            Ok(resolved_path) => {
                // The navigation path is a chain of *aliases* — each segment
                // resolves through the corresponding parent's reference map at
                // eval time, picking up any per-instance reference replacements.
                let submodel_path: Vec<ReferenceName> = submodel_subcomponents
                    .iter()
                    .map(|id| ReferenceName::from(id.as_str()))
                    .collect();

                // Register the extraction on the submodel map keyed by alias.
                resolution_context.add_extracted_submodel_to_active_model(
                    reference_name.clone(),
                    source_name,
                    source_name_span,
                    parent_reference_name.clone(),
                    submodel_path,
                );

                // Also add ReferenceImport with resolved path (for variable validation)
                resolution_context.add_reference_to_active_model(
                    reference_name,
                    reference_name_span,
                    resolved_path,
                );
            }
            Err(error) => {
                resolution_context.add_model_import_resolution_error_to_active_model(
                    reference_name,
                    None,
                    *error,
                );
            }
        }
    }
}

/// Returns the source-level model name and its span — the `foo` in
/// `use foo as bar` (or just `foo` when no alias is given). This is the value
/// stored on `SubmodelImport.name` for diagnostics; the *map key* on the
/// owning model is the alias (a `ReferenceName`), produced separately by
/// [`get_reference_name_and_span`].
fn get_source_name_and_span(model_info: &ast::ModelInfo) -> (SubmodelName, Span) {
    let model_name = model_info.get_model_name();
    let name = SubmodelName::from(model_name.as_str());
    let span = model_name.span();
    (name, span)
}

fn get_reference_name_and_span(model_info: &ast::ModelInfo) -> (ReferenceName, Span) {
    let model_name = model_info.get_alias();
    let name = ReferenceName::from(model_name.as_str());
    let span = model_name.span();
    (name, span)
}

fn calc_import_path(model_path: &ModelPath, model_import: &ast::UseModelNode) -> ModelPath {
    let model_import_relative_path = model_import.get_model_relative_path();
    model_path.get_sibling_model_path(model_import_relative_path)
}

/// Recursively resolves a model path by traversing subcomponents.
///
/// This internal function handles the recursive resolution of model paths
/// when dealing with nested submodels (e.g., `parent.submodel1.submodel2`).
/// It traverses the subcomponent chain and validates that each level exists.
///
/// # Examples
///
/// For a path like `weather.atmosphere.temperature`:
/// 1. First call: `resolve_model_path(None, "weather", ["atmosphere", "temperature"], ...)`
/// 2. Second call: `resolve_model_path(Some("weather"), "atmosphere", ["temperature"], ...)`
/// 3. Third call: `resolve_model_path(Some("atmosphere"), "temperature", [], ...)`
/// 4. Returns: `Ok("temperature")`
///
/// # Panics
///
/// This function assumes that models referenced in `model_info` have been
/// properly loaded and validated. If this assumption is violated, the function
/// will panic, indicating a bug in the model loading process.
fn resolve_model_path<E>(
    model_path: ModelPath,
    model_name_span: Span,
    model_subcomponents: &[ast::IdentifierNode],
    resolution_context: &mut ResolutionContext<'_, E>,
) -> Result<ModelPath, Box<ModelImportResolutionError>>
where
    E: ExternalResolutionContext,
{
    // if the model that we are trying to resolve has had an error, this
    // operation should fail
    let model = match resolution_context.lookup_model(&model_path) {
        ModelResult::Found(model) => model,
        ModelResult::HasError => {
            return Err(Box::new(ModelImportResolutionError::model_has_error(
                model_path,
                model_name_span,
            )));
        }
        ModelResult::NotFound => unreachable!("model should have been visited already"),
    };

    // if there are no more subcomponents, we have resolved the model path
    if model_subcomponents.is_empty() {
        return Ok(model_path);
    }

    // Submodels are keyed by alias (= reference name) on the model, so we
    // navigate dotted paths by alias as well.
    let alias = ReferenceName::from(model_subcomponents[0].as_str());
    let alias_span = model_subcomponents[0].span();
    let submodel_reference = model
        .get_submodel_reference(&alias)
        .ok_or_else(|| {
            let best_match = get_best_match_submodel_alias_in_model(model, &alias);

            ModelImportResolutionError::undefined_submodel_in_submodel(
                model_path,
                SubmodelName::from(alias.as_str()),
                alias_span,
                best_match,
            )
        })?
        .clone();

    let submodel_subcomponents = &model_subcomponents[1..];

    resolve_model_path(
        submodel_reference.path().clone(),
        alias_span,
        submodel_subcomponents,
        resolution_context,
    )
}

fn get_best_match_submodel_alias_in_model(
    model: &ir::Model,
    alias: &ReferenceName,
) -> Option<String> {
    let aliases: Vec<&str> = model
        .get_submodels()
        .keys()
        .map(ReferenceName::as_str)
        .collect();

    search(alias.as_str(), &aliases)
        .and_then(|result| result.some_if_within_distance(MAX_BEST_MATCH_DISTANCE))
        .map(String::from)
}

fn handle_resolution_error<E>(
    error: ModelImportResolutionError,
    model_import: &oneil_ast::Node<oneil_ast::UseModel>,
    reference_name: ReferenceName,
    submodel_name: SubmodelName,
    submodel_name_span: Span,
    is_submodel: bool,
    resolution_context: &mut ResolutionContext<'_, E>,
) where
    E: ExternalResolutionContext,
{
    if is_submodel {
        resolution_context.add_model_import_resolution_error_to_active_model(
            reference_name,
            Some(submodel_name.clone()),
            error,
        );
    } else {
        resolution_context.add_model_import_resolution_error_to_active_model(
            reference_name,
            None,
            error,
        );
    }

    let Some(submodel_list) = model_import.imported_submodels() else {
        // if we don't have any submodels, we're done
        return;
    };

    let parent_model_name = submodel_name;
    let parent_model_name_span = submodel_name_span;

    for submodel_info in submodel_list.iter() {
        // this is a bit hacky, but it's necessary to avoid getting confusing "undefined reference" errors
        let (reference_name, reference_name_span) = get_reference_name_and_span(submodel_info);

        let error = ModelImportResolutionError::parent_model_has_error(
            parent_model_name.clone(),
            parent_model_name_span,
            reference_name.clone(),
            reference_name_span,
        );

        resolution_context.add_model_import_resolution_error_to_active_model(
            reference_name.clone(),
            Some(parent_model_name.clone()),
            error,
        );
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use crate::test::{
        external_context::TestExternalContext, resolution_context::ResolutionContextBuilder,
        test_ast, test_ir, test_model_path, test_model_sibling_path,
    };

    use super::*;
    use oneil_ast as ast;
    use oneil_ir as ir;

    /// Asserts that the submodel map contains exactly the expected direct submodels.
    /// Direct submodels are those with empty `submodel_path` (not extracted via `with`).
    /// The submodel map is keyed by alias (= reference name).
    macro_rules! assert_has_submodels {
        ($submodel_map:expr, $reference_map:expr, $expected_submodels:expr $(,)?) => {
            let submodel_map: &IndexMap<ReferenceName, ir::SubmodelImport> = $submodel_map;
            let reference_map: &IndexMap<ReferenceName, ir::ReferenceImport> = $reference_map;
            let expected_submodels: Vec<(&'static str, &ModelPath)> =
                $expected_submodels.into_iter().collect();

            // Count direct (non-extracted) submodels
            let direct_submodels: Vec<_> = submodel_map
                .iter()
                .filter(|(_, import)| !import.is_extracted())
                .collect();

            // check that the direct submodel count matches expected
            assert_eq!(
                direct_submodels.len(),
                expected_submodels.len(),
                "length of *actual* direct submodel map differs from *expected* submodel map",
            );

            for (alias, expected_path) in expected_submodels {
                let alias = ReferenceName::from(alias);
                let submodel_import = submodel_map.get(&alias).expect(
                    format!("did not find submodel for '{}'", alias.as_str()).as_str(),
                );
                assert!(
                    !submodel_import.is_extracted(),
                    "expected '{}' to be a direct submodel, not extracted",
                    alias.as_str()
                );
                let reference_import = reference_map
                    .get(submodel_import.reference_name())
                    .expect("submodel's reference should exist");

                assert_eq!(
                    reference_import.path(),
                    expected_path,
                    "actual submodel path for '{}' differs from expected",
                    alias.as_str(),
                );
            }
        };
    }

    /// Asserts that the submodel map contains the expected extracted submodels.
    /// Extracted submodels are those created via `with` clauses; each segment
    /// of the navigation path is an alias (= reference name).
    // This is a macro so that assertion failures point to the call site in the
    // test rather than to a line inside a helper function.
    macro_rules! assert_has_extracted_submodels {
        ($submodel_map:expr, $expected_extractions:expr $(,)?) => {
            let submodel_map: &IndexMap<ReferenceName, ir::SubmodelImport> = $submodel_map;
            let expected_extractions: Vec<(&'static str, &'static str, Vec<&'static str>)> =
                $expected_extractions.into_iter().collect();

            // Count extracted submodels
            let extracted_submodels: Vec<_> = submodel_map
                .iter()
                .filter(|(_, import)| import.is_extracted())
                .collect();

            // check that the extracted submodel count matches expected
            assert_eq!(
                extracted_submodels.len(),
                expected_extractions.len(),
                "length of *actual* extracted submodel map differs from *expected*",
            );

            for (alias, parent_ref, expected_path) in expected_extractions {
                let alias = ReferenceName::from(alias);
                let submodel_import = submodel_map.get(&alias).expect(
                    format!(
                        "did not find extracted submodel '{}'",
                        alias.as_str()
                    )
                    .as_str(),
                );
                assert!(
                    submodel_import.is_extracted(),
                    "expected '{}' to be an extracted submodel",
                    alias.as_str()
                );
                assert_eq!(
                    submodel_import.reference_name().as_str(),
                    parent_ref,
                    "parent reference for '{}' differs from expected",
                    alias.as_str()
                );
                let actual_path: Vec<&str> = submodel_import
                    .submodel_path()
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                assert_eq!(
                    actual_path,
                    expected_path,
                    "submodel_path for '{}' differs from expected",
                    alias.as_str()
                );
            }
        };
    }

    // This is a macro, as opposed to a function, because we want the error
    // location to show the line in the test where the assertion failed, rather
    // than some line in an `assert_has_references` function
    macro_rules! assert_has_references {
        ($reference_map:expr, $references:expr $(,)?) => {
            let reference_map: &IndexMap<ReferenceName, ir::ReferenceImport> = $reference_map;
            let references: Vec<(&'static str, &ModelPath)> = $references.into_iter().collect();

            // check that the reference map length is the same as the number of references
            assert_eq!(
                reference_map.len(),
                references.len(),
                "length of *actual* reference map differs from *expected* reference map",
            );

            // check that the reference map contains the expected references
            for (reference_name, reference_path) in references {
                let reference_name = ReferenceName::from(reference_name);
                let reference_import = reference_map.get(&reference_name).expect(
                    format!(
                        "did not find reference path for '{}'",
                        reference_name.as_str()
                    )
                    .as_str(),
                );

                assert_eq!(
                    reference_import.path(),
                    reference_path,
                    "actual reference path for '{}' differs from expected reference path",
                    reference_name.as_str(),
                );
            }
        };
    }

    #[test]
    fn resolve_simple_submodel() {
        // build the model import list:
        // > use temperature as temp
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (temperature at sibling path so lookup finds it)
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(temperature_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (keyed by alias "temp", not model name "temperature")
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the resolved references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_nested_submodel() {
        // build the model import list:
        // > use weather.atmosphere.temperature as temp
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_subcomponents(["atmosphere", "temperature"])
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (models in dependency order; paths as siblings so lookup finds them)
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let atmosphere_path = test_model_sibling_path(&weather_path, "atmosphere");
        let temperature_path = test_model_sibling_path(&atmosphere_path, "temperature");
        let atmosphere_model = test_ir::ModelBuilder::new()
            .with_submodel("temperature", &temperature_path)
            .build();
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("atmosphere", &atmosphere_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (atmosphere_path, atmosphere_model),
                (weather_path, weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (uses alias "temp" as key, not model name "temperature")
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the resolved references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_submodel_without_alias() {
        // build the model import list:
        // > use temperature  # (no alias, reference name is "temperature")
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (temperature at sibling path)
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(temperature_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temperature", &temperature_path)],
        );

        // check the resolved references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temperature", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_submodel_with_subcomponent_alias() {
        // build the model import list:
        // > use weather.atmosphere  # (subcomponent name as alias)
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_subcomponents(["atmosphere"])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (weather and atmosphere at sibling paths)
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let atmosphere_path = test_model_sibling_path(&weather_path, "atmosphere");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("atmosphere", &atmosphere_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (atmosphere_path.clone(), test_ir::empty_model()),
                (weather_path, weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("atmosphere", &atmosphere_path)],
        );

        // check the resolved references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("atmosphere", &atmosphere_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_model_with_error() {
        // build the model import list:
        // > use error_model as error  # (model has error)
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("error_model")
            .with_alias("error")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (error_model at sibling path, marked as having an error)
        let model_path = test_model_path("/parent_model");
        let error_path = test_model_sibling_path(&model_path, "error_model");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(error_path.clone(), test_ir::empty_model())])
            .with_model_errors([error_path.clone()])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (none; import failed)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the resolved references (none; import failed)
        assert_has_references!(resolution_context.get_active_model_references(), []);

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);

        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("error"))
            .expect("error should exist");

        // Submodel name uses alias "error", not model name "error_model"
        assert_eq!(submodel_name, &Some(SubmodelName::from("error")));

        let ModelImportResolutionError::ModelHasError {
            model_path,
            reference_span: _,
        } = error
        else {
            panic!("Expected ModelHasError, got {error:?}");
        };
        assert_eq!(model_path, &error_path);
    }

    #[test]
    fn resolve_undefined_submodel() {
        // build the model import list:
        // > use weather.undefined_submodel  # (weather has no such submodel)
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_subcomponents(["undefined_submodel"])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (weather at sibling path, empty so no undefined_submodel)
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(weather_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (none; import failed)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the resolved references (none; import failed)
        assert_has_references!(resolution_context.get_active_model_references(), []);

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);

        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("undefined_submodel"))
            .expect("error should exist");
        assert_eq!(
            submodel_name,
            &Some(SubmodelName::from("undefined_submodel"))
        );

        let ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span: _,
            best_match: _,
        } = error
        else {
            panic!("Expected UndefinedSubmodel, got {error:?}");
        };
        assert_eq!(parent_model_path, &weather_path);
        assert_eq!(submodel.as_str(), "undefined_submodel");
    }

    #[test]
    fn resolve_undefined_submodel_in_submodel() {
        // build the model import list:
        // > use weather.atmosphere.undefined  # (atmosphere has no "undefined")
        let model_import = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_subcomponents(["atmosphere", "undefined"])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&model_import];

        // build the context (weather and atmosphere at sibling paths; atmosphere has no "undefined")
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let atmosphere_path = test_model_sibling_path(&weather_path, "atmosphere");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("atmosphere", &atmosphere_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (atmosphere_path.clone(), test_ir::empty_model()),
                (weather_path, weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (none; import failed)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the resolved references (none; import failed)
        assert_has_references!(resolution_context.get_active_model_references(), []);

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);

        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("undefined"))
            .expect("error should exist");
        assert_eq!(submodel_name, &Some(SubmodelName::from("undefined")));

        let ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span: _,
            best_match: _,
        } = error
        else {
            panic!("Expected UndefinedSubmodel, got {error:?}");
        };
        assert_eq!(parent_model_path, &atmosphere_path);
        assert_eq!(submodel.as_str(), "undefined");
    }

    #[test]
    fn resolve_multiple_submodels() {
        // build the model import list:
        // > use temperature as temp
        // > use pressure as press
        let temp_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let press_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("pressure")
            .with_alias("press")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&temp_model, &press_model];

        // build the context (temperature and pressure at sibling paths)
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let pressure_path = test_model_sibling_path(&model_path, "pressure");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (pressure_path.clone(), test_ir::empty_model()),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (keyed by aliases, not model names)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path), ("press", &pressure_path),],
        );

        // check the resolved references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path), ("press", &pressure_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_mixed_success_and_error() {
        // build the model import list:
        // > use temperature as temp  # (success)
        // > use error_model as error  # (error)
        let temp_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let error_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("error_model")
            .with_alias("error")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&temp_model, &error_model];

        // build the context (temperature and error_model at sibling paths; error_model marked as having error)
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let error_path = test_model_sibling_path(&model_path, "error_model");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (error_path.clone(), test_ir::empty_model()),
            ])
            .with_model_errors([error_path.clone()])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (only temp, keyed by alias)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the resolved references (only temp)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);

        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("error"))
            .expect("error should exist");
        // Submodel name uses alias "error", not model name "error_model"
        assert_eq!(submodel_name, &Some(SubmodelName::from("error")));

        let ModelImportResolutionError::ModelHasError {
            model_path: err_path,
            reference_span: _,
        } = error
        else {
            panic!("Expected ModelHasError, got {error:?}");
        };
        assert_eq!(err_path, &error_path);
    }

    #[test]
    fn resolve_submodel_with_directory_path_success() {
        // build the model import list:
        // > use utils/math as math  # (directory path)
        let math_model = test_ast::ImportModelNodeBuilder::new()
            .with_directory_path(["utils"])
            .with_top_component("math")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&math_model];

        // build the context (math at sibling path utils/math)
        let model_path = test_model_path("/parent_model");
        let math_path = test_model_sibling_path(&model_path, "utils/math");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(math_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("math", &math_path)],
        );

        // check the resolved references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("math", &math_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_submodel_with_directory_path_error() {
        // build the model import list:
        // > use nonexistent/math as math  # (model has error)
        let math_model = test_ast::ImportModelNodeBuilder::new()
            .with_directory_path(["nonexistent"])
            .with_top_component("math")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&math_model];

        // build the context (math at sibling path nonexistent/math, marked as having error)
        let model_path = test_model_path("/parent_model");
        let math_path = test_model_sibling_path(&model_path, "nonexistent/math");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(math_path.clone(), test_ir::empty_model())])
            .with_model_errors([math_path.clone()])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (none; import failed)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the resolved references (none; import failed)
        assert_has_references!(resolution_context.get_active_model_references(), []);

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);

        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("math"))
            .expect("error should exist");
        assert_eq!(submodel_name, &Some(SubmodelName::from("math")));

        let ModelImportResolutionError::ModelHasError {
            model_path: err_path,
            reference_span: _,
        } = error
        else {
            panic!("Expected ModelHasError, got {error:?}");
        };
        assert_eq!(err_path, &math_path);
    }

    #[test]
    fn resolve_duplicate_submodel_aliases() {
        // build the model import list:
        // > use temperature as temp
        // > use other_temperature as temp  # (duplicate alias)
        let temp_model1 = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let temp_model2 = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("other_temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&temp_model1, &temp_model2];

        // build the context (temperature and other_temperature at sibling paths)
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let other_temperature_path = test_model_sibling_path(&model_path, "other_temperature");
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (other_temperature_path, test_ir::empty_model()),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (only first; second failed due to duplicate alias)
        // Keyed by alias "temp", not model name "temperature"
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the resolved references (only temp -> temperature)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the errors (duplicate reference "temp")
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);
        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("temp"))
            .expect("error should exist");

        // Submodel name uses alias "temp", not model name "other_temperature"
        assert_eq!(submodel_name, &Some(SubmodelName::from("temp")));

        // With aliased submodels, duplicate aliases now trigger DuplicateSubmodel
        // (since both submodels are keyed by "temp")
        let ModelImportResolutionError::DuplicateSubmodel {
            submodel,
            original_span: _,
            duplicate_span: _,
        } = error
        else {
            panic!("Expected DuplicateSubmodel, got {error:?}");
        };
        assert_eq!(submodel.as_str(), "temp");
    }

    #[test]
    fn resolve_use_declaration_with_failing_submodel() {
        // build the model import list:
        // > use weather.atmosphere.temperature  # (atmosphere has no temperature)
        let weather_model_ast = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_subcomponents(["atmosphere", "temperature"])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&weather_model_ast];

        // build the context (weather and atmosphere at sibling paths; atmosphere has no temperature)
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let atmosphere_path = test_model_sibling_path(&weather_path, "atmosphere");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("atmosphere", &atmosphere_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (atmosphere_path.clone(), test_ir::empty_model()),
                (weather_path, weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (none; import failed)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the resolved references (none; import failed)
        assert_has_references!(resolution_context.get_active_model_references(), []);

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);
        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("temperature"))
            .expect("error should exist");
        assert_eq!(submodel_name, &Some(SubmodelName::from("temperature")));

        let ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span: _,
            best_match: _,
        } = error
        else {
            panic!("Expected UndefinedSubmodel, got {error:?}");
        };
        assert_eq!(parent_model_path, &atmosphere_path);
        assert_eq!(submodel.as_str(), "temperature");
    }

    #[test]
    fn resolve_use_declaration_with_successful_and_failing_submodels() {
        // build the model import list:
        // > use temperature as temp  # (success)
        // > use weather.atmosphere.undefined  # (fail)
        let temp_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let undefined_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_subcomponents(["atmosphere", "undefined"])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&temp_model, &undefined_model];

        // build the context (temperature, weather, atmosphere at sibling paths; atmosphere has no "undefined")
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let atmosphere_path = test_model_sibling_path(&weather_path, "atmosphere");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("atmosphere", &atmosphere_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (atmosphere_path.clone(), test_ir::empty_model()),
                (weather_path, weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved submodels (only temp, keyed by alias)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the resolved references (only temp)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);
        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("undefined"))
            .expect("error should exist");
        // No alias, so submodel name is "undefined" (the default alias)
        assert_eq!(submodel_name, &Some(SubmodelName::from("undefined")));

        let ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span: _,
            best_match: _,
        } = error
        else {
            panic!("Expected UndefinedSubmodel, got {error:?}");
        };
        assert_eq!(parent_model_path, &atmosphere_path);
        assert_eq!(submodel.as_str(), "undefined");
    }

    #[test]
    fn resolve_use_declaration_with_single_submodel() {
        // build the model import list:
        // > use weather with temperature as temp
        let temperature_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .build();
        let weather_model_ast = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_submodels([temperature_submodel])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&weather_model_ast];

        // build the context (weather and temperature at sibling paths)
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let temperature_path = test_model_sibling_path(&weather_path, "temperature");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("temperature", &temperature_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (weather_path.clone(), weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved direct submodels (weather as submodel)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("weather", &weather_path)],
        );

        // check extracted submodels (temp extracted from weather via "temperature" path)
        assert_has_extracted_submodels!(
            resolution_context.get_active_model_submodels(),
            [("temp", "weather", vec!["temperature"])],
        );

        // check the resolved references (weather + temp, extracted submodels also create references)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("weather", &weather_path), ("temp", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_use_declaration_with_multiple_submodels() {
        // build the model import list:
        // > use weather with [temperature as temp, pressure as press]
        let temperature_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .build();
        let pressure_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("pressure")
            .with_alias("press")
            .build();
        let use_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_submodels([temperature_submodel, pressure_submodel])
            .with_kind(ast::ModelKind::Submodel)
            .build();
        let model_imports: Vec<&ast::UseModelNode> = vec![&use_model];

        // build the context (weather, temperature, pressure at sibling paths)
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let temperature_path = test_model_sibling_path(&weather_path, "temperature");
        let pressure_path = test_model_sibling_path(&weather_path, "pressure");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("temperature", &temperature_path)
            .with_submodel("pressure", &pressure_path)
            .build();
        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (pressure_path.clone(), test_ir::empty_model()),
                (weather_path.clone(), weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // run the resolution
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the resolved direct submodels (weather as submodel)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("weather", &weather_path)],
        );

        // check extracted submodels (temp and press extracted from weather)
        assert_has_extracted_submodels!(
            resolution_context.get_active_model_submodels(),
            [
                ("temp", "weather", vec!["temperature"]),
                ("press", "weather", vec!["pressure"]),
            ],
        );

        // check the resolved references (weather + temp + press)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [
                ("weather", &weather_path),
                ("temp", &temperature_path),
                ("press", &pressure_path),
            ],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_use_declaration_with_nested_submodel() {
        // create the use model list with a nested submodel in the with clause
        // > use weather with atmosphere.temperature as temp
        let temperature_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("atmosphere")
            .with_subcomponents(["temperature"])
            .with_alias("temp")
            .build();

        let weather_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_submodels([temperature_submodel])
            .with_kind(ast::ModelKind::Submodel)
            .build();

        let model_imports = vec![&weather_model];

        // create the current model path and sibling paths used by the resolver
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let atmosphere_path = test_model_sibling_path(&weather_path, "atmosphere");
        let temperature_path = test_model_sibling_path(&atmosphere_path, "temperature");

        let atmosphere_model = test_ir::ModelBuilder::new()
            .with_submodel("temperature", &temperature_path)
            .build();
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("atmosphere", &atmosphere_path)
            .build();

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (atmosphere_path, atmosphere_model),
                (weather_path.clone(), weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the direct submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("weather", &weather_path)],
        );

        // check extracted submodels (temp extracted via nested path atmosphere.temperature)
        assert_has_extracted_submodels!(
            resolution_context.get_active_model_submodels(),
            [("temp", "weather", vec!["atmosphere", "temperature"])],
        );

        // check the references (weather + temp)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("weather", &weather_path), ("temp", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_use_declaration_with_failing_submodel_in_with_clause() {
        // create the use model list with a failing submodel in the with clause
        // use weather with undefined
        let undefined_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("undefined")
            .build();

        let weather_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_submodels([undefined_submodel])
            .with_kind(ast::ModelKind::Submodel)
            .build();

        let model_imports = vec![&weather_model];

        // create the current model path and sibling path for weather
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(weather_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("weather", &weather_path)],
        );

        // check the references (only weather, undefined failed)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("weather", &weather_path)],
        );

        // check the errors
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);

        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("undefined"))
            .expect("error should exist");
        // Failed extractions don't create SubmodelImport, so submodel_name is None
        assert_eq!(submodel_name, &None);

        let ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span: _,
            best_match: _,
        } = error
        else {
            panic!("Expected UndefinedSubmodel, got {error:?}");
        };

        assert_eq!(parent_model_path, &weather_path);
        assert_eq!(submodel.as_str(), "undefined");
    }

    #[test]
    fn resolve_use_declaration_with_successful_and_failing_submodels_in_with_clause() {
        // create the use model list with both successful and failing submodels in the with clause
        // use weather with [temperature as temp, undefined as undefined]
        let temperature_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .build();
        let undefined_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("undefined")
            .build();
        let weather_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_submodels([temperature_submodel, undefined_submodel])
            .with_kind(ast::ModelKind::Submodel)
            .build();

        let model_imports = vec![&weather_model];

        // create the current model path and sibling paths
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let temperature_path = test_model_sibling_path(&weather_path, "temperature");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("temperature", &temperature_path)
            .build();

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (weather_path.clone(), weather_model),
                (temperature_path.clone(), test_ir::empty_model()),
            ])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the direct submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("weather", &weather_path)],
        );

        // check extracted submodels (temp extracted successfully)
        assert_has_extracted_submodels!(
            resolution_context.get_active_model_submodels(),
            [("temp", "weather", vec!["temperature"])],
        );

        // check the references (weather + temp, undefined failed)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("weather", &weather_path), ("temp", &temperature_path)],
        );

        // check the errors (undefined extraction failed)
        let model_import_errors = resolution_context.get_active_model_model_import_errors();
        assert_eq!(model_import_errors.len(), 1);
        let (submodel_name, error) = model_import_errors
            .get(&ReferenceName::from("undefined"))
            .expect("error should exist");
        // Failed extractions don't create SubmodelImport, so submodel_name is None
        assert_eq!(submodel_name, &None);

        let ModelImportResolutionError::UndefinedSubmodel {
            parent_model_path,
            submodel,
            reference_span: _,
            best_match: _,
        } = error
        else {
            panic!("Expected UndefinedSubmodel, got {error:?}");
        };
        assert_eq!(parent_model_path, &weather_path);
        assert_eq!(submodel.as_str(), "undefined");
    }

    #[test]
    fn resolve_use_declaration_with_model_alias_and_submodels() {
        // create the use model list with model alias and submodels in the with clause
        // use weather as weather_model with [temperature as temp, pressure as press]
        let temperature_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .build();
        let pressure_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("pressure")
            .with_alias("press")
            .build();
        let use_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("weather")
            .with_alias("weather_model")
            .with_submodels([temperature_submodel, pressure_submodel])
            .with_kind(ast::ModelKind::Submodel)
            .build();

        let import_models = vec![&use_model];

        // create the current model path and sibling paths
        let model_path = test_model_path("/parent_model");
        let weather_path = test_model_sibling_path(&model_path, "weather");
        let temperature_path = test_model_sibling_path(&weather_path, "temperature");
        let pressure_path = test_model_sibling_path(&weather_path, "pressure");
        let weather_model = test_ir::ModelBuilder::new()
            .with_submodel("temperature", &temperature_path)
            .with_submodel("pressure", &pressure_path)
            .build();

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (temperature_path.clone(), test_ir::empty_model()),
                (pressure_path.clone(), test_ir::empty_model()),
                (weather_path.clone(), weather_model),
            ])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, import_models, &mut resolution_context);

        // check the direct submodels (keyed by alias "weather_model", not model name "weather")
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [("weather_model", &weather_path)],
        );

        // check extracted submodels (temp and press extracted from weather_model)
        assert_has_extracted_submodels!(
            resolution_context.get_active_model_submodels(),
            [
                ("temp", "weather_model", vec!["temperature"]),
                ("press", "weather_model", vec!["pressure"]),
            ],
        );

        // check the references (weather_model + temp + press)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [
                ("weather_model", &weather_path),
                ("temp", &temperature_path),
                ("press", &pressure_path),
            ],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_reference() {
        // create the import model list
        // > ref temperature
        let temp_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_kind(ast::ModelKind::Reference)
            .build();

        let model_imports = vec![&temp_model];

        // create the current model path and sibling path for the ref target
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(temperature_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temperature", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_reference_with_alias() {
        // create the import model list
        // > ref temperature as temp
        let temp_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Reference)
            .build();

        let model_imports = vec![&temp_model];

        // create the current model path and sibling path for the ref target
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([(temperature_path.clone(), test_ir::empty_model())])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the submodels
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check the references
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }

    #[test]
    fn resolve_reference_with_alias_and_submodels() {
        // create the import model list
        // > ref temperature as temp with [pressure as press]
        let pressure_submodel = test_ast::ModelInfoNodeBuilder::new()
            .with_top_component("pressure")
            .with_alias("press")
            .build();

        let temp_model = test_ast::ImportModelNodeBuilder::new()
            .with_top_component("temperature")
            .with_alias("temp")
            .with_kind(ast::ModelKind::Reference)
            .with_submodels([pressure_submodel])
            .build();

        let model_imports = vec![&temp_model];

        // create the current model path and sibling paths (ref target temperature, then pressure under it)
        let model_path = test_model_path("/parent_model");
        let temperature_path = test_model_sibling_path(&model_path, "temperature");
        let pressure_path = test_model_sibling_path(&temperature_path, "pressure");
        let temperature_model = test_ir::ModelBuilder::new()
            .with_submodel("pressure", &pressure_path)
            .build();

        let mut external = TestExternalContext::new();
        let mut resolution_context = ResolutionContextBuilder::new()
            .with_active_model(model_path.clone())
            .with_models([
                (pressure_path.clone(), test_ir::empty_model()),
                (temperature_path.clone(), temperature_model),
            ])
            .with_external_context(&mut external)
            .build();

        // resolve the submodels
        resolve_model_imports(&model_path, model_imports, &mut resolution_context);

        // check the direct submodels (none, since ref doesn't create a submodel)
        assert_has_submodels!(
            resolution_context.get_active_model_submodels(),
            resolution_context.get_active_model_references(),
            [],
        );

        // check extracted submodels (press extracted from temp)
        assert_has_extracted_submodels!(
            resolution_context.get_active_model_submodels(),
            [("press", "temp", vec!["pressure"])],
        );

        // check the references (temp + press)
        assert_has_references!(
            resolution_context.get_active_model_references(),
            [("temp", &temperature_path), ("press", &pressure_path)],
        );

        // check the errors
        assert!(
            resolution_context
                .get_active_model_model_import_errors()
                .is_empty()
        );
    }
}
