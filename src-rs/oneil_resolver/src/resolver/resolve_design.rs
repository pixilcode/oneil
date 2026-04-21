//! Resolution of design surface declarations: `design <model>`,
//! `apply <file> to <ref>(.<ref>)*`, and design parameter assignments
//! (`id(.<ref>)* = expr`).
//!
//! `apply` resolves sibling **`.one`** files; `design <model>` resolves the
//! target as a sibling **`.on`** model.

use std::ops::Deref;

use indexmap::IndexSet;
use oneil_ast as ast;
use oneil_ir as ir;
use oneil_shared::{
    InstancePath,
    labels::ParameterLabel,
    paths::ModelPath,
    span::Span,
    symbols::{ParameterName, ReferenceName},
};

use crate::{
    ExternalResolutionContext, ResolutionContext, context::ModelResult, resolver::resolve_parameter,
};

/// Loads sibling models referenced by `apply` declarations so their IR exists
/// before resolution.
pub fn preload_design_files<E: ExternalResolutionContext>(
    model_path: &ModelPath,
    model_ast: &ast::Model,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    if let Some(target_path) = collect_design_target_path(model_path, model_ast) {
        super::load_model(&target_path, resolution_context);
    }
    for path in collect_apply_design_paths(model_path, model_ast) {
        super::load_model(&path, resolution_context);
    }
}

/// Renders a [`ModelPath`] for inclusion in user-facing messages.
fn display_model_path(path: &ModelPath) -> String {
    let raw = path.as_path();
    if let Some(name) = raw.file_name().and_then(|s| s.to_str()) {
        return name.to_string();
    }
    raw.display().to_string()
}

/// Returns `true` when two [`ModelPath`]s point at the same on-disk file,
/// canonicalizing where possible so equivalent paths that differ syntactically
/// (e.g. one with `..` segments) compare equal.
fn same_model_path(a: &ModelPath, b: &ModelPath) -> bool {
    if a == b {
        return true;
    }
    match (a.as_path().canonicalize(), b.as_path().canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
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

/// Collects unique paths referenced by every `apply` declaration (including nested ones).
#[must_use]
pub fn collect_apply_design_paths(
    model_path: &ModelPath,
    model_ast: &ast::Model,
) -> IndexSet<ModelPath> {
    let mut out = IndexSet::new();
    for ad in iter_apply_designs(model_ast) {
        let relative_path = ad.get_design_relative_path();
        out.insert(model_path.get_sibling_design_path(relative_path));
    }
    out
}

/// Walks design-related declarations in source order (top level, then each section).
#[derive(Debug, Clone, Copy)]
pub enum DesignSurfaceItem<'a> {
    Target(&'a ast::DesignTargetNode),
    /// `apply <file> to <path>(. <ref>)* [ … ]` declaration on this model.
    Apply(&'a ast::ApplyDesignNode),
    /// `id(.<ref>)* = expr` parameter line in a design file.
    Parameter(&'a ast::DesignParameterNode),
}

/// Walks all declarations (top-level and within sections) producing a
/// [`DesignSurfaceItem`] for each design-related entry.
pub fn collect_design_surface(model_ast: &ast::Model) -> Vec<DesignSurfaceItem<'_>> {
    let mut items = Vec::new();

    let all_decls = model_ast
        .decls()
        .iter()
        .chain(model_ast.sections().iter().flat_map(|s| s.decls().iter()));

    for decl in all_decls {
        match &**decl {
            ast::Decl::DesignTarget(n) => items.push(DesignSurfaceItem::Target(n)),
            ast::Decl::ApplyDesign(n) => items.push(DesignSurfaceItem::Apply(n)),
            ast::Decl::DesignParameter(n) => items.push(DesignSurfaceItem::Parameter(n)),
            ast::Decl::Import(_)
            | ast::Decl::UseModel(_)
            | ast::Decl::Parameter(_)
            | ast::Decl::Test(_) => {}
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

/// Iterates over every apply declaration in `model_ast`, including nested ones.
fn iter_apply_designs(model_ast: &ast::Model) -> Vec<&ast::ApplyDesignNode> {
    let mut out = Vec::new();
    for item in collect_design_surface(model_ast) {
        if let DesignSurfaceItem::Apply(n) = item {
            push_apply_recursive(n, &mut out);
        }
    }
    out
}

fn push_apply_recursive<'a>(
    node: &'a ast::ApplyDesignNode,
    out: &mut Vec<&'a ast::ApplyDesignNode>,
) {
    out.push(node);
    for nested in node.nested_applies() {
        push_apply_recursive(nested, out);
    }
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
/// 4. Record an [`ir::ApplyDesign`] for every `apply <file> to <path>` so the
///    instancing pass can compose contributions at evaluation time.
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
            DesignSurfaceItem::Apply(_) => {
                // Apply declarations are recorded later by `record_applied_designs`,
                // not folded into the design's own export.
            }
        }
    }

    let exported_target = explicit_target.clone();
    store_design_export(&surface, explicit_target, running, resolution_context);
    record_applied_designs(
        &surface,
        model_path,
        exported_target.as_ref(),
        resolution_context,
    );
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
            // Only flat parameters can introduce new design-local names.
            DesignSurfaceItem::Parameter(p) if p.instance_path().is_empty() => {
                design_param_names.insert(ParameterName::from(p.ident().as_str()));
            }
            DesignSurfaceItem::Parameter(_) | DesignSurfaceItem::Apply(_) => {}
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
/// existing target parameter, as a scoped override on a reference path, or as
/// a new design-local parameter.
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

    // Scoped parameters (`param.ref(.ref)* = …`) carry an instance path suffix.
    let instance_path = if p.instance_path().is_empty() {
        None
    } else {
        let mut path = InstancePath::root();
        for seg in p.instance_path() {
            path = path.child(ReferenceName::new(seg.as_str().to_string()));
        }
        Some(path)
    };

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

/// Stores the final [`ir::Design`] on the active model if the surface produced
/// any design content; otherwise stores an empty export.
fn store_design_export<E: ExternalResolutionContext>(
    surface: &[DesignSurfaceItem<'_>],
    explicit_target: Option<ModelPath>,
    mut running: ir::Design,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    let has_design_content = explicit_target.is_some()
        || surface
            .iter()
            .any(|i| matches!(i, DesignSurfaceItem::Parameter(_)));

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

/// Records a declarative [`ir::ApplyDesign`] for every `apply <file> to <path>`
/// (including nested entries) after validating that the target path is
/// resolvable and that the design's declared target matches the model that the
/// path resolves to. The actual stamping happens in the instancing pass during
/// evaluation.
fn record_applied_designs<E: ExternalResolutionContext>(
    surface: &[DesignSurfaceItem<'_>],
    model_path: &ModelPath,
    explicit_target: Option<&ModelPath>,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    let consuming_model = explicit_target
        .cloned()
        .unwrap_or_else(|| model_path.clone());

    for item in surface {
        let DesignSurfaceItem::Apply(node) = item else {
            continue;
        };
        record_apply_recursive(
            node,
            &InstancePath::root(),
            model_path,
            &consuming_model,
            resolution_context,
        );
    }
}

/// Records `node` (and its nested applies) as concrete [`ir::ApplyDesign`]
/// entries. The `outer_target` is the path accumulated from outer apply blocks
/// (root for top-level applies); the resolved `target` is `outer_target`
/// concatenated with the apply's own segments.
fn record_apply_recursive<E: ExternalResolutionContext>(
    node: &ast::ApplyDesignNode,
    outer_target: &InstancePath,
    model_path: &ModelPath,
    consuming_model: &ModelPath,
    resolution_context: &mut ResolutionContext<'_, E>,
) {
    // Resolve each segment of the apply target through the two-tier lookup:
    // first by reference name, then by unique model name. The first segment is
    // resolved against `consuming_model`; subsequent segments require resolving
    // through the live reference graph and are not supported yet — they
    // produce an error.
    let segments = node.target();
    let Some((first_seg, rest_segs)) = segments.split_first() else {
        // Parser guarantees a non-empty target; treat as a no-op defensively.
        return;
    };

    let resolved_first =
        match resolve_segment_in(consuming_model, first_seg.as_str(), resolution_context) {
            Ok(name) => name,
            Err(err) => {
                resolution_context
                    .add_design_resolution_error_to_active_model(err, first_seg.span());
                return;
            }
        };

    if !rest_segs.is_empty() {
        // Multi-segment apply targets across child instances are not yet
        // supported by the resolver. Emit a clear error so users know.
        resolution_context.add_design_resolution_error_to_active_model(
            format!(
                "multi-segment apply target `{}.{}` is not yet supported",
                first_seg.as_str(),
                rest_segs
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            first_seg.span(),
        );
        return;
    }

    let target_path = outer_target.clone().child(resolved_first.clone());

    // Validate the design's declared target matches the model the first
    // segment resolves to, when the apply lives at the root of the consuming
    // model (i.e. `outer_target` is the root). Deeper validation is skipped
    // because we don't yet model child-of-child reference graphs at
    // resolution time.
    let relative_path = node.get_design_relative_path();
    let dpath = model_path.get_sibling_design_path(relative_path);

    if outer_target.is_root()
        && let Some(referenced_model_path) =
            lookup_referenced_model_path(consuming_model, &resolved_first, resolution_context)
    {
        let design_target = match resolution_context.lookup_model(&dpath) {
            ModelResult::Found(m) => m.design_export().target_model.clone(),
            ModelResult::HasError | ModelResult::NotFound => None,
        };
        if let Some(design_target) = design_target.as_ref()
            && !same_model_path(design_target, &referenced_model_path)
        {
            resolution_context.add_design_resolution_error_to_active_model(
                format!(
                    "`apply {design} to {ref_name}`: design `{design}` targets `{design_target}`, \
                     but `{ref_name}` resolves to `{ref_target}`. \
                     Use a design whose `design <model>` matches `{ref_target}`.",
                    ref_name = resolved_first.as_str(),
                    design = display_model_path(&dpath),
                    design_target = display_model_path(design_target),
                    ref_target = display_model_path(&referenced_model_path),
                ),
                node.span(),
            );
            return;
        }

        // Surface augmented parameter names so `ref.new_param` lookups
        // succeed during resolution.
        let augmented = match resolution_context.lookup_model(&dpath) {
            ModelResult::Found(m) => {
                let d = m.design_export();
                if d.parameter_additions.is_empty() {
                    None
                } else {
                    Some(d.clone())
                }
            }
            ModelResult::HasError | ModelResult::NotFound => None,
        };
        if let Some(d) = augmented {
            resolution_context.add_augmented_reference_to_active_model(resolved_first.clone(), d);
        }
    }

    resolution_context
        .active_model_mut()
        .add_applied_design(ir::ApplyDesign {
            design_path: dpath,
            target: target_path.clone(),
            span: node.span(),
        });

    for nested in node.nested_applies() {
        record_apply_recursive(
            nested,
            &target_path,
            model_path,
            consuming_model,
            resolution_context,
        );
    }
}

/// Resolves a single apply/extraction segment against `model_path` using the
/// two-tier lookup: first by reference name, then by unique model name among
/// references. Returns the resolved [`ReferenceName`] on success, or an error
/// message describing the lookup failure.
fn resolve_segment_in<E: ExternalResolutionContext>(
    model_path: &ModelPath,
    segment: &str,
    resolution_context: &ResolutionContext<'_, E>,
) -> Result<ReferenceName, String> {
    let model = match resolution_context.lookup_model(model_path) {
        ModelResult::Found(m) => m,
        ModelResult::HasError | ModelResult::NotFound => {
            return Err(format!(
                "cannot resolve `{}` because target model `{}` failed to load",
                segment,
                display_model_path(model_path),
            ));
        }
    };

    let candidate = ReferenceName::new(segment.to_string());
    if model.get_references().contains_key(&candidate) {
        return Ok(candidate);
    }

    // Fall back to looking the segment up by underlying model file name.
    let matches: Vec<&ReferenceName> = model
        .get_references()
        .iter()
        .filter(|(_, import)| {
            import.path().as_path().file_stem().and_then(|s| s.to_str()) == Some(segment)
        })
        .map(|(name, _)| name)
        .collect();

    match matches.as_slice() {
        [] => Err(format!(
            "no reference named `{segment}` (and no reference whose model is named `{segment}`) on `{model}`",
            segment = segment,
            model = display_model_path(model_path),
        )),
        [single] => Ok((*single).clone()),
        _ => Err(format!(
            "segment `{segment}` is ambiguous on `{model}`: matches references {refs}",
            segment = segment,
            model = display_model_path(model_path),
            refs = matches
                .iter()
                .map(|r| format!("`{}`", r.as_str()))
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

/// Returns the model path that reference `rn` resolves to from the perspective
/// of `consuming_model` (typically a model file or a design's target).
///
/// Returns `None` when the reference cannot be resolved (missing, has an
/// upstream error, etc.).
fn lookup_referenced_model_path<E: ExternalResolutionContext>(
    consuming_model: &ModelPath,
    rn: &ReferenceName,
    resolution_context: &ResolutionContext<'_, E>,
) -> Option<ModelPath> {
    let model = match resolution_context.lookup_model(consuming_model) {
        ModelResult::Found(m) => m,
        ModelResult::HasError | ModelResult::NotFound => return None,
    };
    model.get_references().get(rn).map(|r| r.path().clone())
}
