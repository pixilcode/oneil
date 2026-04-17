//! The instance graph: a single pre-computed structure of all model instances
//! reachable from a root, with reference replacements, design overlays, and
//! design-introduced parameters already resolved to absolute coordinates.
//!
//! Building the graph is the **single place** where design composition happens:
//! - Reference replacements from `use model as alias` are baked into each
//!   instance's reference wiring.
//! - Parameter overrides from `use design X [for ref]` land as
//!   [`OverlayBinding`] entries keyed by `(EvalInstanceKey, ParameterName)`.
//! - Parameter additions from a design are merged into the target instance's
//!   parameter set.
//! - Scoped overrides (`x.ref = …`) propagate down the live tree via the
//!   reference structure — including post-replacement references.
//!
//! Once built, the [`crate::EvalContext`] just consumes the graph: forcing
//! pending parameters, evaluating tests, and propagating reference errors.

use indexmap::{IndexMap, IndexSet};
use oneil_ir as ir;
use oneil_shared::{
    EvalInstanceKey, InstancePath,
    load_result::LoadResult,
    paths::ModelPath,
    span::Span,
    symbols::{ParameterName, ReferenceName, SubmodelName},
};

use crate::context::ExternalEvaluationContext;

/// One model instance in the [`InstanceGraph`].
///
/// All design contributions targeting this instance have already been baked in
/// (parameters include design additions, references reflect post-replacement
/// targets, overlays hold the resolved RHS keyed by parameter name).
#[derive(Debug, Clone)]
pub struct InstancedModel {
    /// On-disk model file backing this instance (post-replacement).
    pub model_path: ModelPath,
    /// All declared parameters on this instance: own IR parameters plus any
    /// parameter additions from designs landed here.
    pub parameters: IndexMap<ParameterName, ir::Parameter>,
    /// References on this instance, mapped to the child instance they point to
    /// (post-replacement). Includes both true references and synthetic
    /// references from extracted submodels.
    pub references: IndexMap<ReferenceName, EvalInstanceKey>,
    /// Submodel-name → reference-name aliases (both `use` direct submodels and
    /// `with`-extracted submodels).
    pub submodels: IndexMap<SubmodelName, ReferenceName>,
    /// Tests on this instance (copied from the IR).
    pub tests: IndexMap<oneil_shared::symbols::TestIndex, ir::Test>,
    /// Overlays that apply to this instance, keyed by parameter name.
    pub overlays: IndexMap<ParameterName, OverlayBinding>,
}

/// One overlay binding on an [`InstancedModel`].
///
/// Holds the resolved overlay RHS plus the [`anchor_key`](Self::anchor_key)
/// — the instance whose lexical scope owns the overlay (the design landed
/// there). The RHS is evaluated in the anchor's scope, not the target's.
#[derive(Debug, Clone)]
pub struct OverlayBinding {
    /// The resolved RHS to use in place of the IR default.
    pub value: ir::ParameterValue,
    /// Span of the design assignment that produced this overlay.
    pub design_span: Span,
    /// Span of the original parameter definition on the target model IR.
    pub original_model_span: Span,
    /// Instance whose lexical scope owns this overlay.
    pub anchor_key: EvalInstanceKey,
}

/// The whole instance tree reachable from a root model, fully resolved.
#[derive(Debug, Clone)]
pub struct InstanceGraph {
    /// Root instance the graph was built from.
    pub root: EvalInstanceKey,
    /// Every instance reachable from the root.
    pub instances: IndexMap<EvalInstanceKey, InstancedModel>,
}

impl InstanceGraph {
    /// Builds the graph by walking from `root_path` once, applying any
    /// `runtime_designs` at the root in addition to designs declared by the
    /// model files themselves.
    ///
    /// Shared refs (those resolving to the same [`EvalInstanceKey`]) are
    /// visited exactly once; their first visitor's landed design contributions
    /// are the ones that take effect, matching prior behavior.
    pub fn build<E: ExternalEvaluationContext>(
        root_path: &ModelPath,
        runtime_designs: &[ir::DesignApplication],
        external: &E,
    ) -> Self {
        let root_key = EvalInstanceKey {
            model_path: root_path.clone(),
            instance_path: InstancePath::root(),
        };

        let mut graph = Self {
            root: root_key.clone(),
            instances: IndexMap::new(),
        };
        let mut visited = IndexSet::new();

        // Convert runtime-supplied applications into landed contributions for
        // the root and dispatch the build. Anchor for runtime designs is the
        // root instance — that's the lexical scope CLI `--design` flags adopt.
        let mut root_landings: Vec<DesignContribution> = Vec::new();
        for app in runtime_designs {
            if let Some(contribution) = contribution_for_application(app, &root_key, external) {
                root_landings.push(contribution);
            }
        }

        visit(&root_key, root_landings, &mut graph, &mut visited, external);
        graph
    }
}

/// Per-instance accumulation of design contributions while walking the tree.
///
/// A `DesignContribution` is the part of a [`Design`](ir::Design) that lands at
/// a particular instance. The `anchor_key` records the lexical scope of the
/// design — overrides forwarded into nested instances retain that anchor so
/// their RHS expressions still bind to the design's own scope at eval time.
///
/// `nested_by_ref` carries the parts that should be forwarded to specific
/// child references; that propagation is what lets a `scoped_overrides` entry
/// like `x.child.grandchild = …` end up as a plain override on the deep
/// grandchild instance.
#[derive(Debug, Clone)]
struct DesignContribution {
    /// Lexical scope of the design that produced this contribution. Preserved
    /// when forwarding through `nested_by_ref`.
    anchor_key: EvalInstanceKey,
    /// Parameter overrides landing on this instance.
    overrides: IndexMap<ParameterName, ir::OverlayParameterValue>,
    /// New parameters this design adds to the target.
    additions: IndexMap<ParameterName, ir::Parameter>,
    /// Reference replacements this design declares.
    replacements: IndexMap<ReferenceName, ir::ReferenceReplacement>,
    /// Contributions to forward to specific child references.
    nested_by_ref: IndexMap<ReferenceName, Self>,
}

impl DesignContribution {
    fn empty(anchor_key: EvalInstanceKey) -> Self {
        Self {
            anchor_key,
            overrides: IndexMap::new(),
            additions: IndexMap::new(),
            replacements: IndexMap::new(),
            nested_by_ref: IndexMap::new(),
        }
    }

    /// Lifts a [`Design`](ir::Design) into a contribution anchored at
    /// `anchor_key`, splitting `scoped_overrides` by their first path segment
    /// for propagation.
    fn from_design(design: &ir::Design, anchor_key: &EvalInstanceKey) -> Self {
        let mut out = Self {
            anchor_key: anchor_key.clone(),
            overrides: design.parameter_overrides.clone(),
            additions: design.parameter_additions.clone(),
            replacements: design.reference_replacements.clone(),
            nested_by_ref: IndexMap::new(),
        };
        for (path, params) in &design.scoped_overrides {
            push_scoped(&mut out, path.segments(), params, anchor_key);
        }
        out
    }
}

/// Inserts a scoped-override entry into the right place inside `target`,
/// drilling through `nested_by_ref` until `segments` is empty. New nested
/// nodes inherit the design's `anchor_key` so the override RHS can still be
/// evaluated in the design's lexical scope.
fn push_scoped(
    target: &mut DesignContribution,
    segments: &[ReferenceName],
    params: &IndexMap<ParameterName, ir::OverlayParameterValue>,
    anchor_key: &EvalInstanceKey,
) {
    if let Some((first, rest)) = segments.split_first() {
        let child = target
            .nested_by_ref
            .entry(first.clone())
            .or_insert_with(|| DesignContribution::empty(anchor_key.clone()));
        if rest.is_empty() {
            for (k, v) in params {
                child.overrides.insert(k.clone(), v.clone());
            }
        } else {
            push_scoped(child, rest, params, anchor_key);
        }
    } else {
        // Empty path: treat as a root-level override.
        for (k, v) in params {
            target.overrides.insert(k.clone(), v.clone());
        }
    }
}

/// Resolves a [`DesignApplication`](ir::DesignApplication) to a
/// [`DesignContribution`]: looks up the design IR and lifts it, anchoring it at
/// the consuming instance (the model whose `use design` declaration applied
/// the design). If the application targets a specific reference, the
/// contribution is wrapped so it lands on that reference's child instance —
/// the anchor stays at the consuming instance, which is the design's lexical
/// scope.
///
/// Returns `None` if the design IR cannot be loaded.
fn contribution_for_application<E: ExternalEvaluationContext>(
    app: &ir::DesignApplication,
    consuming_key: &EvalInstanceKey,
    external: &E,
) -> Option<DesignContribution> {
    let design = load_design(&app.design_path, external)?;
    let mut contribution = DesignContribution::from_design(&design, consuming_key);
    if let Some(ref_name) = &app.applied_to {
        let mut wrapped = DesignContribution::empty(consuming_key.clone());
        wrapped.nested_by_ref.insert(ref_name.clone(), contribution);
        contribution = wrapped;
    }
    Some(contribution)
}

/// Loads a design IR from `external`. Returns `None` if the design isn't
/// loaded or has no value.
fn load_design<E: ExternalEvaluationContext>(
    design_path: &ModelPath,
    external: &E,
) -> Option<ir::Design> {
    let load = external.lookup_ir(design_path)?;
    let model = match load {
        LoadResult::Success(m) | LoadResult::Partial(m, _) => m,
        LoadResult::Failure => return None,
    };
    Some(model.design_export().clone())
}

/// Combines a model's IR parameters with `parameter_additions` from each
/// landed design (later landings win).
fn compose_parameters(
    own: &IndexMap<ParameterName, ir::Parameter>,
    landed: &[DesignContribution],
) -> IndexMap<ParameterName, ir::Parameter> {
    let mut out = own.clone();
    for c in landed {
        for (name, param) in &c.additions {
            out.insert(name.clone(), param.clone());
        }
    }
    out
}

/// Builds the per-instance overlay table from landed contributions, preserving
/// each override's anchor (the design's lexical scope, not the target).
fn compose_overlays(landed: &[DesignContribution]) -> IndexMap<ParameterName, OverlayBinding> {
    let mut out: IndexMap<ParameterName, OverlayBinding> = IndexMap::new();
    for c in landed {
        for (name, ov) in &c.overrides {
            out.insert(
                name.clone(),
                OverlayBinding {
                    value: ov.value.clone(),
                    design_span: ov.design_span,
                    original_model_span: ov.original_model_span,
                    anchor_key: c.anchor_key.clone(),
                },
            );
        }
    }
    out
}

/// Collects reference-name → replacement-path entries from landed
/// contributions (later landings win).
fn collect_replacements(landed: &[DesignContribution]) -> IndexMap<ReferenceName, ModelPath> {
    let mut out: IndexMap<ReferenceName, ModelPath> = IndexMap::new();
    for c in landed {
        for (name, repl) in &c.replacements {
            out.insert(name.clone(), repl.replacement_path.clone());
        }
    }
    out
}

/// DFS that materializes one instance and recurses into its children.
fn visit<E: ExternalEvaluationContext>(
    key: &EvalInstanceKey,
    landed: Vec<DesignContribution>,
    graph: &mut InstanceGraph,
    visited: &mut IndexSet<EvalInstanceKey>,
    external: &E,
) {
    if !visited.insert(key.clone()) {
        return;
    }

    let Some(model_load) = external.lookup_ir(&key.model_path) else {
        return;
    };
    let model = match model_load {
        LoadResult::Success(m) | LoadResult::Partial(m, _) => m,
        LoadResult::Failure => return,
    };

    // Add this model's own `applied_designs` into the landed contributions.
    // Their anchor is the consuming instance (this `key`).
    let mut all_landed = landed;
    for app in model.applied_designs() {
        if let Some(c) = contribution_for_application(app, key, external) {
            all_landed.push(c);
        }
    }

    let parameters = compose_parameters(model.get_parameters(), &all_landed);
    let overlays = compose_overlays(&all_landed);
    let replacements = collect_replacements(&all_landed);

    // Build the references map (post-replacement) and submodel aliases.
    let model_refs = model.get_references();
    let model_subs = model.get_submodels();

    let mut references: IndexMap<ReferenceName, EvalInstanceKey> = IndexMap::new();
    for (ref_name, ref_import) in model_refs {
        let child_path = replacements
            .get(ref_name)
            .cloned()
            .unwrap_or_else(|| ref_import.path().clone());
        let is_direct_submodel = model_subs
            .values()
            .any(|s| s.reference_name() == ref_name && !s.is_extracted());
        let child_instance = if is_direct_submodel {
            key.instance_path.child(ref_name.clone())
        } else {
            InstancePath::root()
        };
        let child_key = EvalInstanceKey {
            model_path: child_path,
            instance_path: child_instance,
        };
        references.insert(ref_name.clone(), child_key);
    }

    let mut submodels: IndexMap<SubmodelName, ReferenceName> = model_subs
        .iter()
        .map(|(name, import)| (name.clone(), import.reference_name().clone()))
        .collect();

    // Wire extracted submodels (`with` clauses) — synthetic references on this
    // instance that point at navigation-derived child instances.
    let mut extracted_targets: Vec<(ReferenceName, EvalInstanceKey)> = Vec::new();
    for (sub_name, sub_import) in model_subs {
        if !sub_import.is_extracted() {
            continue;
        }
        // Parent reference must already be in `references` (we just built it).
        let Some(parent_key) = references.get(sub_import.reference_name()).cloned() else {
            continue;
        };
        let child_path =
            navigate_submodel_path(&parent_key.model_path, sub_import.submodel_path(), external);
        let child_instance = key
            .instance_path
            .child(ReferenceName::from(sub_name.as_str()));
        let extracted_ref = ReferenceName::from(sub_name.as_str());
        let child_key = EvalInstanceKey {
            model_path: child_path,
            instance_path: child_instance,
        };
        references.insert(extracted_ref.clone(), child_key.clone());
        // Make the extracted submodel name visible as a submodel alias too,
        // so that `submodels` callers can find it without special-casing.
        submodels
            .entry(sub_name.clone())
            .or_insert_with(|| extracted_ref.clone());
        extracted_targets.push((extracted_ref, child_key));
    }

    let tests = model.get_tests().clone();

    graph.instances.insert(
        key.clone(),
        InstancedModel {
            model_path: key.model_path.clone(),
            parameters,
            references: references.clone(),
            submodels,
            tests,
            overlays,
        },
    );

    // Recurse into each true reference, forwarding only the matching nested
    // contributions to that child.
    for (ref_name, child_key) in references {
        let child_landed: Vec<DesignContribution> = all_landed
            .iter()
            .filter_map(|c| c.nested_by_ref.get(&ref_name).cloned())
            .collect();
        visit(&child_key, child_landed, graph, visited, external);
    }

    // Extracted submodels recurse with no inherited contributions (mirrors
    // current behavior — `with` extractions don't carry overlays through).
    for (_extracted_ref, child_key) in extracted_targets {
        visit(&child_key, Vec::new(), graph, visited, external);
    }
}

/// Navigates a chain of submodel names from `parent_path`, returning the
/// terminal model path. Mirrors the previous evaluator helper.
fn navigate_submodel_path<E: ExternalEvaluationContext>(
    parent_path: &ModelPath,
    submodel_path: &[SubmodelName],
    external: &E,
) -> ModelPath {
    let mut current_path = parent_path.clone();
    for submodel_name in submodel_path {
        let Some(load) = external.lookup_ir(&current_path) else {
            return current_path;
        };
        let model_ir = match load {
            LoadResult::Success(m) | LoadResult::Partial(m, _) => m,
            LoadResult::Failure => return current_path,
        };
        let Some(submodel) = model_ir.get_submodel(submodel_name) else {
            return current_path;
        };
        let Some(reference) = model_ir.get_reference(submodel.reference_name()) else {
            return current_path;
        };
        current_path = reference.path().clone();
    }
    current_path
}
