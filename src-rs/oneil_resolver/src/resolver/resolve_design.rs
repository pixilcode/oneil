//! Resolution of `design`, `use design`, and design shorthand assignments.
//!
//! `use design` resolves sibling **`.one`** files; `design <model>` still resolves the target as a sibling **`.on`** model.

use std::ops::Deref;

use indexmap::IndexSet;
use oneil_ast as ast;
use oneil_ir as ir;
use oneil_shared::{
    InstancePath,
    labels::ParameterLabel,
    paths::ModelPath,
    span::Span,
    symbols::{ParameterName, ReferenceName, SubmodelName},
};

use crate::{
    ExternalResolutionContext, ResolutionContext,
    context::{ModelResult, ReferencePathResult},
    resolver::resolve_parameter,
};

/// Loads sibling models referenced by `use design` and design reference replacements
/// so their IR exists before resolution.
pub fn preload_design_files<E: ExternalResolutionContext>(
    model_path: &ModelPath,
    model_ast: &ast::Model,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    // Load the design target model (the model specified in `design <target>`)
    if let Some(target_path) = collect_design_target_path(model_path, model_ast) {
        super::load_model(&target_path, resolution_context);
    }
    // Load design files
    for path in collect_use_design_paths(model_path, model_ast) {
        super::load_model(&path, resolution_context);
    }
    // Load reference replacement models
    for path in collect_reference_replacement_paths(model_path, model_ast) {
        super::load_model(&path, resolution_context);
    }
}

/// Returns the path to the design target model if there's a `design <target>` declaration.
fn collect_design_target_path(model_path: &ModelPath, model_ast: &ast::Model) -> Option<ModelPath> {
    for item in collect_design_surface(model_ast) {
        if let DesignSurfaceItem::Target(node) = item {
            let relative_path = node.get_target_relative_path();
            return Some(model_path.get_sibling_model_path(relative_path));
        }
    }
    None
}

/// Collects unique paths for every `use design` declaration.
#[must_use]
pub fn collect_use_design_paths(
    model_path: &ModelPath,
    model_ast: &ast::Model,
) -> IndexSet<ModelPath> {
    let mut out = IndexSet::new();
    for ud in iter_use_designs(model_ast) {
        // UseDesign supports full paths via get_design_relative_path()
        let relative_path = ud.get_design_relative_path();
        let p = model_path.get_sibling_design_path(relative_path);
        out.insert(p);
    }
    out
}

/// Collects unique paths for every design reference replacement (`use model as alias`).
#[must_use]
fn collect_reference_replacement_paths(
    model_path: &ModelPath,
    model_ast: &ast::Model,
) -> IndexSet<ModelPath> {
    let mut out = IndexSet::new();
    for item in collect_design_surface(model_ast) {
        if let DesignSurfaceItem::Reference(um) = item {
            // UseModel supports full paths via get_model_relative_path()
            let relative_path = um.get_model_relative_path();
            let p = model_path.get_sibling_model_path(relative_path);
            out.insert(p);
        }
    }
    out
}

/// Walks design-related declarations in source order (top level, then each section).
#[derive(Debug, Clone, Copy)]
pub enum DesignSurfaceItem<'a> {
    Target(&'a ast::DesignTargetNode),
    UseDesign(&'a ast::UseDesignNode),
    Parameter(&'a ast::DesignParameterNode),
    /// Reference replacement: `use model as alias` in design context.
    /// The `UseModel` must have an alias set.
    Reference(&'a ast::UseModelNode),
}

/// Checks if a `UseModel` is a reference replacement.
///
/// A reference replacement is a `use model as alias` declaration (not `ref`).
/// It must use the `use` keyword (Submodel kind) and have an explicit alias.
pub fn is_reference_replacement(um: &ast::UseModel) -> bool {
    um.model_kind() == ast::ModelKind::Submodel && um.model_info().alias().is_some()
}

pub fn collect_design_surface(model_ast: &ast::Model) -> Vec<DesignSurfaceItem<'_>> {
    let mut items = Vec::new();
    let mut pending_use_models: Vec<&ast::UseModelNode> = Vec::new();
    let mut has_design_target = false;

    let all_decls = model_ast
        .decls()
        .iter()
        .chain(model_ast.sections().iter().flat_map(|s| s.decls().iter()));

    for decl in all_decls {
        match &**decl {
            ast::Decl::DesignTarget(n) => {
                has_design_target = true;
                items.push(DesignSurfaceItem::Target(n));
            }
            ast::Decl::UseDesign(n) => items.push(DesignSurfaceItem::UseDesign(n)),
            ast::Decl::DesignParameter(n) => items.push(DesignSurfaceItem::Parameter(n)),
            ast::Decl::UseModel(n) if is_reference_replacement(n) => {
                pending_use_models.push(n);
            }
            ast::Decl::Import(_)
            | ast::Decl::UseModel(_)
            | ast::Decl::Parameter(_)
            | ast::Decl::Test(_) => {}
        }
    }

    // `use model as alias` is only a reference replacement when this file is a
    // design file (has a `design <target>` declaration).
    if has_design_target {
        for um in pending_use_models {
            items.push(DesignSurfaceItem::Reference(um));
        }
    }

    items
}

/// Returns the span of the full parameter definition on `model_path`, or `fallback` if missing.
fn span_of_parameter_on_model<E: ExternalResolutionContext>(
    ctx: &ResolutionContext<'_, E>,
    model_path: &ModelPath,
    param: &ParameterName,
    fallback: Span,
) -> Span {
    match ctx.lookup_model(model_path) {
        ModelResult::Found(m) => m.get_parameter(param).map_or(fallback, ir::Parameter::span),
        ModelResult::HasError | ModelResult::NotFound => fallback,
    }
}

fn iter_use_designs(model_ast: &ast::Model) -> impl Iterator<Item = &ast::UseDesignNode> + '_ {
    collect_design_surface(model_ast)
        .into_iter()
        .filter_map(|item| match item {
            DesignSurfaceItem::UseDesign(n) => Some(n),
            DesignSurfaceItem::Target(_)
            | DesignSurfaceItem::Parameter(_)
            | DesignSurfaceItem::Reference(_) => None,
        })
}

/// Registers a design-local parameter as a scratch entry on the resolution context.
///
/// This lets design-local parameters reference each other during resolution without
/// polluting the target model's IR. The scratch entry is only visible to parameter
/// lookups; the real `ir::Parameter` (with its resolved value) is stored in the
    /// design's `parameter_additions`.
fn register_design_local_scratch<E: ExternalResolutionContext>(
    model_path: &ModelPath,
    name: ParameterName,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    use oneil_shared::span::SourceLocation;

    let scratch_loc = SourceLocation {
        offset: 0,
        line: 0,
        column: 0,
    };
    let scratch_span = Span::empty(scratch_loc);

    let scratch = ir::Parameter::new(
        ir::Dependencies::new(),
        name.clone(),
        scratch_span,
        scratch_span,
        ParameterLabel::from(name.as_str()),
        None,
        ir::ParameterValue::Simple(
            ir::Expr::literal(scratch_span, ir::Literal::Number(0.0)),
            None,
        ),
        ir::Limits::default(),
        false,
        ir::TraceLevel::None,
        None,
    );
    resolution_context.register_design_local_parameter(model_path.clone(), name, scratch);
}

/// Resolves the design surface for the active model.
///
/// Orchestrates four stages over the items collected by [`collect_design_surface`]:
///
/// 1. Scan for the design target and register scratch entries for design-local
///    parameters so they can cross-reference each other during resolution.
/// 2. Dispatch each surface item to the appropriate handler, accumulating into
///    a running [`ir::Design`].
/// 3. Store the resulting design export on the active model.
/// 4. Record a [`ir::DesignApplication`] for every `use design … for <ref>` so
///    the instancing pass can compose contributions at evaluation time.
pub fn resolve_design_surface<E: ExternalResolutionContext>(
    model_path: &ModelPath,
    model_ast: &ast::Model,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    let surface = collect_design_surface(model_ast);

    let (mut explicit_target, design_local_param_names) =
        scan_design_locals(model_path, &surface, resolution_context);

    let mut running = ir::Design::new();
    let mut design_target_declared = false;

    for item in &surface {
        match item {
            DesignSurfaceItem::Target(node) => handle_design_target(
                node,
                model_path,
                &mut explicit_target,
                &mut design_target_declared,
                &mut running,
                resolution_context,
            ),
            DesignSurfaceItem::Parameter(p) => handle_design_parameter(
                p,
                explicit_target.as_ref(),
                &design_local_param_names,
                &mut running,
                resolution_context,
            ),
            DesignSurfaceItem::UseDesign(ud) => handle_use_design(
                ud,
                model_path,
                explicit_target.as_ref(),
                &mut running,
                resolution_context,
            ),
            DesignSurfaceItem::Reference(um) => handle_reference_replacement(
                um,
                model_path,
                explicit_target.as_ref(),
                &mut running,
                resolution_context,
            ),
        }
    }

    store_design_export(&surface, explicit_target, running, resolution_context);
    record_applied_designs(&surface, model_path, resolution_context);
}

/// Scans the design surface for the target declaration and design-local
/// parameter names, registering scratch entries so design-local parameters
/// can reference each other during resolution.
///
/// Returns `(explicit_target, design_local_param_names)`.
fn scan_design_locals<E: ExternalResolutionContext>(
    model_path: &ModelPath,
    surface: &[DesignSurfaceItem<'_>],
    resolution_context: &mut ResolutionContext<'_, E>,
) -> (Option<ModelPath>, IndexSet<ParameterName>) {
    let mut explicit_target: Option<ModelPath> = None;
    let mut design_param_names: IndexSet<ParameterName> = IndexSet::new();
    for item in surface {
        match item {
            DesignSurfaceItem::Target(node) => {
                let relative_path = node.get_target_relative_path();
                explicit_target = Some(model_path.get_sibling_model_path(relative_path));
            }
            // Only non-scoped parameters can introduce new design-local names.
            DesignSurfaceItem::Parameter(p) if p.instance().is_none() => {
                design_param_names.insert(ParameterName::from(p.ident().as_str()));
            }
            DesignSurfaceItem::Parameter(_)
            | DesignSurfaceItem::UseDesign(_)
            | DesignSurfaceItem::Reference(_) => {}
        }
    }

    let mut design_local_param_names: IndexSet<ParameterName> = IndexSet::new();
    if let Some(tgt) = &explicit_target {
        for name in &design_param_names {
            let exists = matches!(
                resolution_context.lookup_model(tgt),
                ModelResult::Found(m) if m.get_parameter(name).is_some()
            );
            if !exists {
                design_local_param_names.insert(name.clone());
                register_design_local_scratch(tgt, name.clone(), resolution_context);
            }
        }
    }

    (explicit_target, design_local_param_names)
}

/// Handles a `design <model>` declaration; also updates `running.target_model`.
fn handle_design_target<E: ExternalResolutionContext>(
    node: &ast::DesignTargetNode,
    model_path: &ModelPath,
    explicit_target: &mut Option<ModelPath>,
    design_target_declared: &mut bool,
    running: &mut ir::Design,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    if *design_target_declared {
        resolution_context.add_design_resolution_error_to_active_model(
            "only one `design` declaration is allowed per file",
            node.span(),
        );
        return;
    }
    *design_target_declared = true;
    let relative_path = node.get_target_relative_path();
    let p = model_path.get_sibling_model_path(relative_path);
    *explicit_target = Some(p.clone());
    running.target_model = Some(p);
}

/// Handles a design parameter line: either records it as an override on an
/// existing target parameter, as a scoped override on a reference, or as a
/// new design-local parameter.
fn handle_design_parameter<E: ExternalResolutionContext>(
    p: &ast::DesignParameterNode,
    explicit_target: Option<&ModelPath>,
    design_local_param_names: &IndexSet<ParameterName>,
    running: &mut ir::Design,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    let Some(tgt) = explicit_target.cloned() else {
        resolution_context.add_design_resolution_error_to_active_model(
            "design parameter line requires a preceding `design <model>` declaration",
            p.ident().span(),
        );
        return;
    };
    let name = ParameterName::from(p.ident().as_str());

    // Scoped parameters (`param.ref = …`) carry an instance suffix.
    let instance_path = p.instance().map(|inst| {
        let ref_name = ReferenceName::new(inst.as_str().to_string());
        InstancePath::root().child(ref_name)
    });

    // Resolve the RHS in the design target's scope so names inside it bind to
    // the target's parameters (and our pre-registered design-local scratches).
    let active = resolution_context
        .active_models()
        .last()
        .expect("active model");
    let pushed = active != &tgt;
    if pushed {
        resolution_context.push_active_model(&tgt);
    }
    let resolved_value =
        resolve_parameter::resolve_parameter_value(p.value().deref(), resolution_context);
    if pushed {
        resolution_context.pop_active_model(&tgt);
    }

    let value = match resolved_value {
        Ok(v) => v,
        Err(errs) => {
            for e in errs {
                resolution_context.add_parameter_error_to_active_model(name.clone(), e);
            }
            return;
        }
    };

    let design_span = p.ident().span();

    // `design_local_param_names` was populated before we registered scratch
    // entries, so it reflects names that weren't on the target model's IR.
    // Scoped overrides always target an existing parameter, never a local one.
    let is_local_param = instance_path.is_none() && design_local_param_names.contains(&name);

    if is_local_param {
        let dependencies =
            resolve_parameter::get_parameter_dependencies(&value, &ir::Limits::default());
        let label = ParameterLabel::from(p.ident().as_str());
        let local_param = ir::Parameter::new(
            dependencies,
            name.clone(),
            design_span,
            design_span,
            label,
            None,
            value,
            ir::Limits::default(),
            false,
            ir::TraceLevel::None,
            None,
        );
        running.parameter_additions.insert(name, local_param);
        return;
    }

    let original_model_span =
        span_of_parameter_on_model(resolution_context, &tgt, &name, design_span);
    let overlay_value = ir::OverlayParameterValue {
        value,
        design_span,
        original_model_span,
    };
    match instance_path {
        Some(ip) => {
            running
                .scoped_overrides
                .entry(ip)
                .or_default()
                .insert(name, overlay_value);
        }
        None => {
            running.parameter_overrides.insert(name, overlay_value);
        }
    }
}

/// Handles a `use design <file> [for <ref>]` declaration inside a design
/// surface, merging the imported bundle into `running` and (for `for <ref>`)
/// recording its augmented-reference parameters.
fn handle_use_design<E: ExternalResolutionContext>(
    ud: &ast::UseDesignNode,
    model_path: &ModelPath,
    explicit_target: Option<&ModelPath>,
    running: &mut ir::Design,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    let relative_path = ud.get_design_relative_path();
    let dpath = model_path.get_sibling_design_path(relative_path);
    let imported_design = match resolution_context.lookup_model(&dpath) {
        ModelResult::Found(m) => m.design_export().clone(),
        ModelResult::HasError | ModelResult::NotFound => return,
    };

    match ud.instance() {
        None => {
            if explicit_target.is_none() {
                resolution_context.add_design_resolution_error_to_active_model(
                    "`use design` without `for` requires a preceding `design <model>` declaration",
                    ud.span(),
                );
                return;
            }
            running.merge_later_wins(&imported_design);
        }
        Some(id_node) => {
            let rn = ReferenceName::new(id_node.as_str().to_string());
            let prefix = InstancePath::root().child(rn.clone());
            running.merge_prefixed(&prefix, &imported_design);

            // Record augmented params so `ref.new_param` lookups succeed during
            // resolution. The instancing pass consumes them at eval time.
            if !imported_design.parameter_additions.is_empty() {
                resolution_context
                    .add_augmented_reference_to_active_model(rn, imported_design.clone());
            }
        }
    }
}

/// Handles a reference replacement (`use model as alias [with [submodels]]`)
/// declared inside a design file.
fn handle_reference_replacement<E: ExternalResolutionContext>(
    um: &ast::UseModelNode,
    model_path: &ModelPath,
    explicit_target: Option<&ModelPath>,
    running: &mut ir::Design,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    // `is_reference_replacement` guarantees `model_info().alias()` is `Some`.
    let alias_node = um.model_info().get_alias();
    let Some(tgt) = explicit_target.cloned() else {
        resolution_context.add_design_resolution_error_to_active_model(
            "reference replacement requires a preceding `design <model>` declaration",
            alias_node.span(),
        );
        return;
    };
    let alias = ReferenceName::new(alias_node.as_str().to_string());

    // A design file may replace a reference defined in the target model, but
    // it may not both create a new reference with this alias and then replace
    // it in the same file.
    if let Some(original_span) = resolution_context
        .get_reference_from_active_model(&alias)
        .map(|r| *r.name_span())
    {
        resolution_context.add_design_resolution_error_to_active_model(
            format!(
                "cannot replace `{}` in the same file where it is defined; \
                 reference replacements should target references in the design target model",
                alias.as_str()
            ),
            alias_node.span(),
        );
        resolution_context.add_design_resolution_error_to_active_model(
            format!("`{}` is defined here", alias.as_str()),
            original_span,
        );
        return;
    }

    let relative_path = um.get_model_relative_path();
    let replacement_path = model_path.get_sibling_model_path(relative_path);

    let design_submodels: Vec<SubmodelName> = um
        .imported_submodels()
        .map(|sl| {
            sl.iter()
                .map(|mi: &ast::ModelInfoNode| {
                    let info: &ast::ModelInfo = mi;
                    SubmodelName::from(info.get_model_name().as_str())
                })
                .collect()
        })
        .unwrap_or_default();

    validate_reference_replacement_submodels(
        resolution_context,
        &tgt,
        &alias,
        &replacement_path,
        &design_submodels,
        alias_node.span(),
    );

    running
        .reference_replacements
        .insert(alias, ir::ReferenceReplacement { replacement_path });
}

/// Stores the final [`ir::Design`] on the active model if the surface produced
/// any design content; otherwise stores an empty export.
fn store_design_export<E: ExternalResolutionContext>(
    surface: &[DesignSurfaceItem<'_>],
    explicit_target: Option<ModelPath>,
    mut running: ir::Design,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    let has_design_content = explicit_target.is_some()
        || surface.iter().any(|i| {
            matches!(
                i,
                DesignSurfaceItem::Parameter(_)
                    | DesignSurfaceItem::UseDesign(_)
                    | DesignSurfaceItem::Reference(_)
            )
        });

    if has_design_content {
        resolution_context
            .active_model_mut()
            .set_design_target(explicit_target.clone());
        running.target_model = explicit_target;
        resolution_context
            .active_model_mut()
            .set_design_export(running);
    } else {
        resolution_context
            .active_model_mut()
            .set_design_export(ir::Design::new());
    }
}

/// Records a declarative [`ir::DesignApplication`] for every
/// `use design … for <ref>` after validating that the reference exists and
/// the design file loaded successfully. The actual stamping happens in the
/// instancing pass during evaluation.
fn record_applied_designs<E: ExternalResolutionContext>(
    surface: &[DesignSurfaceItem<'_>],
    model_path: &ModelPath,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    for item in surface {
        let DesignSurfaceItem::UseDesign(ud) = item else {
            continue;
        };
        let Some(id_node) = ud.instance() else {
            continue;
        };

        let rn = ReferenceName::new(id_node.as_str().to_string());
        if !matches!(
            resolution_context.lookup_reference_path_in_active_model(&rn),
            ReferencePathResult::Found(..)
        ) {
            resolution_context.add_design_resolution_error_to_active_model(
                format!(
                    "`use design … for {}`: reference `{}` not found on this model",
                    rn.as_str(),
                    rn.as_str()
                ),
                id_node.span(),
            );
            continue;
        }

        let relative_path = ud.get_design_relative_path();
        let dpath = model_path.get_sibling_design_path(relative_path);

        // Skip applications targeting a design we couldn't load — diagnostics
        // for that come from elsewhere.
        if !matches!(
            resolution_context.lookup_model(&dpath),
            ModelResult::Found(_)
        ) {
            continue;
        }

        resolution_context
            .active_model_mut()
            .add_applied_design(ir::DesignApplication {
                design_path: dpath,
                applied_to: Some(rn),
                span: ud.span(),
            });
    }
}

/// Validates that submodel extractions in a reference replacement are compatible.
///
/// This checks:
/// 1. If the design specifies `with` submodels, they must exist on the replacement model.
/// 2. If the original reference had `with` extractions, they must also exist on the replacement.
fn validate_reference_replacement_submodels<E: ExternalResolutionContext>(
    resolution_context: &mut ResolutionContext<'_, E>,
    target_model: &ModelPath,
    alias: &ReferenceName,
    replacement_path: &ModelPath,
    design_submodels: &[SubmodelName],
    error_span: Span,
) {
    // Collect all data we need first, then emit errors (avoids borrow checker issues)
    let mut errors: Vec<String> = Vec::new();

    // Look up the target model to find original reference and its submodels
    let (original_ref_path, original_submodels) = {
        let target_ir = match resolution_context.lookup_model(target_model) {
            ModelResult::Found(m) => m,
            ModelResult::HasError | ModelResult::NotFound => return,
        };

        // Find the original reference being replaced
        let Some(original_ref) = target_ir.get_reference(alias) else {
            errors.push(format!(
                "reference `{}` not found on target model `{}`",
                alias.as_str(),
                target_model.as_path().display()
            ));
            // Emit errors before returning
            for err in errors {
                resolution_context.add_design_resolution_error_to_active_model(&err, error_span);
            }
            return;
        };

        // Collect submodels that were imported from the original reference
        let original_submodels: Vec<SubmodelName> = target_ir
            .get_submodels()
            .iter()
            .filter(|(_, import)| import.reference_name() == alias)
            .map(|(name, _)| name.clone())
            .collect();

        (original_ref.path().clone(), original_submodels)
    };

    // Collect parameter names from replacement model
    let replacement_params: Vec<String> = {
        let replacement_ir = match resolution_context.lookup_model(replacement_path) {
            ModelResult::Found(m) => m,
            ModelResult::HasError | ModelResult::NotFound => return,
        };
        replacement_ir
            .get_parameters()
            .keys()
            .map(|p: &oneil_shared::symbols::ParameterName| p.as_str().to_string())
            .collect()
    };

    // Collect parameter names from original referenced model
    let original_model_params: Vec<String> = {
        match resolution_context.lookup_model(&original_ref_path) {
            ModelResult::Found(m) => m
                .get_parameters()
                .keys()
                .map(|p: &oneil_shared::symbols::ParameterName| p.as_str().to_string())
                .collect(),
            ModelResult::HasError | ModelResult::NotFound => Vec::new(),
        }
    };

    // Determine which submodels need to be checked:
    // - If design specifies `with`, use those
    // - Otherwise, use the original submodels that were imported
    let required_submodels = if design_submodels.is_empty() {
        &original_submodels
    } else {
        design_submodels
    };

    // Validate each required submodel exists on the replacement model
    for submodel_name in required_submodels {
        let name_str = submodel_name.as_str();
        // Check if the original model has this parameter (which would be the submodel)
        // The submodel must exist on the replacement model
        if original_model_params.iter().any(|p| p == name_str)
            && !replacement_params.iter().any(|p| p == name_str)
        {
            errors.push(format!(
                "replacement model `{}` does not have parameter `{}` required by original reference",
                replacement_path.as_path().display(),
                name_str
            ));
        }
    }

    // Also check that any design-specified submodels exist on replacement
    for submodel_name in design_submodels {
        let name_str = submodel_name.as_str();
        if !replacement_params.iter().any(|p| p == name_str) {
            errors.push(format!(
                "`with` submodel `{}` does not exist on replacement model `{}`",
                name_str,
                replacement_path.as_path().display()
            ));
        }
    }

    // Emit all collected errors
    for err in errors {
        resolution_context.add_design_resolution_error_to_active_model(&err, error_span);
    }
}
