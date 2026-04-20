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
    symbols::{ParameterName, ReferenceName},
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
    /// (post-replacement). Includes both true references and aliases from
    /// extracted submodels — extracted aliases reuse the existing
    /// [`EvalInstanceKey`] of the deep instance they navigate to, so overlays
    /// applied to that instance via any path are observed consistently.
    pub references: IndexMap<ReferenceName, EvalInstanceKey>,
    /// Aliases of submodel imports declared on this model file (both
    /// direct `use … as alias` submodels and `with`-extracted submodels).
    /// Each element is the same alias used as a key in
    /// [`Self::references`]; the set is provided so consumers can preserve
    /// the submodel-vs-reference distinction.
    pub submodels: IndexSet<ReferenceName>,
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
    /// `anchor_key`, splitting all three scoped maps (`scoped_overrides`,
    /// `scoped_replacements`, `scoped_additions`) by their first path segment
    /// for propagation through `nested_by_ref`.
    fn from_design(design: &ir::Design, anchor_key: &EvalInstanceKey) -> Self {
        let mut out = Self {
            anchor_key: anchor_key.clone(),
            overrides: design.parameter_overrides.clone(),
            additions: design.parameter_additions.clone(),
            replacements: design.reference_replacements.clone(),
            nested_by_ref: IndexMap::new(),
        };
        for (path, params) in &design.scoped_overrides {
            let leaf = descend_or_create(&mut out, path.segments(), anchor_key);
            for (k, v) in params {
                leaf.overrides.insert(k.clone(), v.clone());
            }
        }
        for (path, repls) in &design.scoped_replacements {
            let leaf = descend_or_create(&mut out, path.segments(), anchor_key);
            for (k, v) in repls {
                leaf.replacements.insert(k.clone(), v.clone());
            }
        }
        for (path, adds) in &design.scoped_additions {
            let leaf = descend_or_create(&mut out, path.segments(), anchor_key);
            for (k, v) in adds {
                leaf.additions.insert(k.clone(), v.clone());
            }
        }
        out
    }
}

/// Walks down `target.nested_by_ref` along `segments`, creating empty
/// intermediate contributions as needed, and returns a mutable reference to
/// the leaf. New nodes inherit the supplied `anchor_key` so that anything
/// landed at the leaf preserves the design's lexical scope.
fn descend_or_create<'a>(
    target: &'a mut DesignContribution,
    segments: &[ReferenceName],
    anchor_key: &EvalInstanceKey,
) -> &'a mut DesignContribution {
    let Some((first, rest)) = segments.split_first() else {
        return target;
    };
    let child = target
        .nested_by_ref
        .entry(first.clone())
        .or_insert_with(|| DesignContribution::empty(anchor_key.clone()));
    descend_or_create(child, rest, anchor_key)
}

/// Rewrites a contribution's `nested_by_ref` so that any entry keyed by a
/// `with`-extracted alias on the consuming model is spliced into the
/// extraction chain — `nested_by_ref[parent_ref].nested_by_ref[seg1]…[segN]`.
///
/// Applied recursively into each retained child so multi-level overrides like
/// `a.b.c = …` are also rewritten when `a` happens to be extracted.
fn rewrite_extracted_aliases(
    contribution: &mut DesignContribution,
    submodels: &IndexMap<ReferenceName, ir::SubmodelImport>,
) {
    let extracted_keys: Vec<ReferenceName> = contribution
        .nested_by_ref
        .keys()
        .filter(|name| {
            submodels
                .get(*name)
                .is_some_and(ir::SubmodelImport::is_extracted)
        })
        .cloned()
        .collect();

    // Each extracted alias is replaced with its extraction chain in turn.
    // Deeper levels of nesting belong to descendant models and are rewritten
    // when those models are visited (each visit() rewrites contributions in
    // its own model's namespace).
    for alias in extracted_keys {
        let Some(import) = submodels.get(&alias) else {
            continue;
        };
        let Some(child) = contribution.nested_by_ref.shift_remove(&alias) else {
            continue;
        };
        let mut chain: Vec<ReferenceName> = vec![import.reference_name().clone()];
        chain.extend(import.submodel_path().iter().cloned());
        splice_into_chain(contribution, &chain, child);
    }
}

/// Splices `payload` into `target.nested_by_ref` along `chain`, merging into
/// any pre-existing entries at intermediate steps.
fn splice_into_chain(
    target: &mut DesignContribution,
    chain: &[ReferenceName],
    payload: DesignContribution,
) {
    let anchor = payload.anchor_key.clone();
    let mut cursor = target;
    let Some((last, prefix)) = chain.split_last() else {
        return;
    };
    for seg in prefix {
        cursor = cursor
            .nested_by_ref
            .entry(seg.clone())
            .or_insert_with(|| DesignContribution::empty(anchor.clone()));
    }
    match cursor.nested_by_ref.shift_remove(last) {
        Some(existing) => {
            let merged = merge_contributions(existing, payload);
            cursor.nested_by_ref.insert(last.clone(), merged);
        }
        None => {
            cursor.nested_by_ref.insert(last.clone(), payload);
        }
    }
}

/// Merges two contributions targeting the same instance. `b` wins for
/// conflicting keys (mirrors the "later landings win" semantics elsewhere).
fn merge_contributions(mut a: DesignContribution, b: DesignContribution) -> DesignContribution {
    for (k, v) in b.overrides {
        a.overrides.insert(k, v);
    }
    for (k, v) in b.additions {
        a.additions.insert(k, v);
    }
    for (k, v) in b.replacements {
        a.replacements.insert(k, v);
    }
    for (k, v) in b.nested_by_ref {
        match a.nested_by_ref.shift_remove(&k) {
            Some(existing) => {
                a.nested_by_ref.insert(k, merge_contributions(existing, v));
            }
            None => {
                a.nested_by_ref.insert(k, v);
            }
        }
    }
    a
}

/// Resolves a [`DesignApplication`](ir::DesignApplication) to a
/// [`DesignContribution`]: looks up the design IR and lifts it, anchoring it
/// at the design's target instance — the place where the design's overlays
/// and additions logically live. The anchor matters for overlay RHS
/// evaluation (see [`OverlayBinding::anchor_key`]): a design's RHSes refer
/// to parameters in its target's lexical scope, not in the consumer's.
///
/// - For `use design X` (no `for`): the target instance is the consumer
///   itself, so the anchor is `consuming_key`.
/// - For `use design X for r`: the target instance is `r` on the consumer
///   (a child instance for direct submodels, a shared instance for `ref`s).
///   The contribution is wrapped under `nested_by_ref[r]` so it lands on
///   that instance, but the anchor inside the wrap points at the target
///   instance so RHSes resolve against the target's parameters.
///
/// Returns `None` if the design IR cannot be loaded.
fn contribution_for_application<E: ExternalEvaluationContext>(
    app: &ir::DesignApplication,
    consuming_key: &EvalInstanceKey,
    external: &E,
) -> Option<DesignContribution> {
    let design = load_design(&app.design_path, external)?;
    let inner_anchor = app.applied_to.as_ref().map_or_else(
        || consuming_key.clone(),
        |ref_name| {
            target_instance_key_for(&design, ref_name, consuming_key, external)
                .unwrap_or_else(|| consuming_key.clone())
        },
    );
    let mut contribution = DesignContribution::from_design(&design, &inner_anchor);
    if let Some(ref_name) = &app.applied_to {
        let mut wrapped = DesignContribution::empty(consuming_key.clone());
        wrapped.nested_by_ref.insert(ref_name.clone(), contribution);
        contribution = wrapped;
    }
    Some(contribution)
}

/// Computes the [`EvalInstanceKey`] of a design's target instance when it is
/// applied via `use design X for ref_name` on the consumer at `consuming_key`.
///
/// The design's target model comes from the design IR; the instance path is
/// derived from how the consumer declares `ref_name`:
/// - direct submodel (`use M as ref`): the target instance lives at
///   `consuming_key.instance_path.child(ref_name)`.
/// - shared reference (`ref M as ref`): the target instance lives at the
///   root instance path (shared instances are not nested under the consumer).
///
/// Returns `None` if the design has no declared target, the consumer model IR
/// can't be loaded, or `ref_name` isn't declared on the consumer (resolution
/// emits diagnostics for the latter; we just fall back to `consuming_key`
/// upstream so eval doesn't crash on already-flagged input).
fn target_instance_key_for<E: ExternalEvaluationContext>(
    design: &ir::Design,
    ref_name: &ReferenceName,
    consuming_key: &EvalInstanceKey,
    external: &E,
) -> Option<EvalInstanceKey> {
    let target_model = design.target_model.clone()?;
    let consumer_ir = match external.lookup_ir(&consuming_key.model_path)? {
        LoadResult::Success(m) | LoadResult::Partial(m, _) => m,
        LoadResult::Failure => return None,
    };
    let is_direct_submodel = consumer_ir.get_submodels().contains_key(ref_name);
    let instance_path = if is_direct_submodel {
        consuming_key.instance_path.child(ref_name.clone())
    } else if consumer_ir.get_references().contains_key(ref_name) {
        InstancePath::root()
    } else {
        return None;
    };
    Some(EvalInstanceKey {
        model_path: target_model,
        instance_path,
    })
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

    let model_refs = model.get_references();
    let model_subs = model.get_submodels();

    // Re-route contributions that target a `with`-extracted alias through
    // their extraction chain (e.g. `value.inner = …` becomes
    // `mid.inner.value = …` when `inner` is extracted via `mid.inner`), so
    // that when forwarded to children the contribution reaches the same deep
    // instance that the alias is wired to point at.
    for c in &mut all_landed {
        rewrite_extracted_aliases(c, model_subs);
    }

    let parameters = compose_parameters(model.get_parameters(), &all_landed);
    let overlays = compose_overlays(&all_landed);
    let replacements = collect_replacements(&all_landed);

    // Build the post-replacement references map for *true* references first.
    // Extracted submodels are wired afterwards by walking the live graph so
    // they reuse the same `EvalInstanceKey` as the deep instance.

    // The submodel map is keyed by alias; an alias points at a *direct*
    // submodel iff its `SubmodelImport.submodel_path` is empty.
    let mut submodels: IndexSet<ReferenceName> = model_subs
        .iter()
        .filter(|(_, import)| !import.is_extracted())
        .map(|(alias, _)| alias.clone())
        .collect();

    let mut references: IndexMap<ReferenceName, EvalInstanceKey> = IndexMap::new();
    for (ref_name, ref_import) in model_refs {
        let child_path = replacements
            .get(ref_name)
            .cloned()
            .unwrap_or_else(|| ref_import.path().clone());
        let is_direct_submodel = submodels.contains(ref_name);
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

    let tests = model.get_tests().clone();

    // Insert the partial instance now (with just true references) so that
    // descendants can see it during recursion — the extracted-submodel
    // wiring updates `references` and `submodels` in place after children
    // have been visited.
    graph.instances.insert(
        key.clone(),
        InstancedModel {
            model_path: key.model_path.clone(),
            parameters,
            references: references.clone(),
            submodels: submodels.clone(),
            tests,
            overlays,
        },
    );

    // Recurse into each true reference, forwarding only the matching nested
    // contributions to that child. After this loop, every transitive
    // descendant has been added to `graph.instances`.
    for (ref_name, child_key) in &references {
        let child_landed: Vec<DesignContribution> = all_landed
            .iter()
            .filter_map(|c| c.nested_by_ref.get(ref_name).cloned())
            .collect();
        visit(child_key, child_landed, graph, visited, external);
    }

    // Wire extracted submodels (`with` clauses) by walking the *live* graph
    // references chain — each segment is an alias that the parent's
    // already-built instance has resolved (with replacements applied). The
    // extracted alias on this instance reuses the existing
    // `EvalInstanceKey` of the deep instance, so overlays applied via any
    // path to that instance are observed here.
    for (alias, sub_import) in model_subs {
        if !sub_import.is_extracted() {
            continue;
        }
        let Some(parent_key) = references.get(sub_import.reference_name()).cloned() else {
            continue;
        };
        let mut current_key = parent_key;
        let mut resolved = true;
        for segment in sub_import.submodel_path() {
            let Some(current_inst) = graph.instances.get(&current_key) else {
                resolved = false;
                break;
            };
            let Some(next_key) = current_inst.references.get(segment).cloned() else {
                resolved = false;
                break;
            };
            current_key = next_key;
        }
        if resolved {
            references.insert(alias.clone(), current_key);
            submodels.insert(alias.clone());
        }
    }

    // Patch the previously-inserted instance with the now-complete
    // references and submodels (including any `with`-extracted aliases).
    if let Some(inst) = graph.instances.get_mut(key) {
        inst.references = references;
        inst.submodels = submodels;
    }
}
