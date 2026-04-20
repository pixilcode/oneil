//! Designs and their declarative content.
//!
//! A [`Design`] is the resolved, declarative form of a `.one` design file:
//! parameter overrides for an existing target, parameter additions that
//! introduce new parameters, and reference replacements. Designs are merged
//! and consumed by the instancing pass that wires up the live model tree.

use indexmap::IndexMap;
use oneil_shared::{
    InstancePath,
    paths::ModelPath,
    span::Span,
    symbols::{ParameterName, ReferenceName},
};

use crate::{Parameter, ParameterValue};

/// A design applied by a model file via `use design X [for ref]`.
///
/// A `DesignApplication` is a *declarative* record: it says which design file
/// is applied and, optionally, which reference of the consuming model it
/// targets. Stamping the design's overrides, replacements, and additions onto
/// the live tree happens in the instancing pass — not at resolution time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignApplication {
    /// Path to the `.one` design file applied here.
    pub design_path: ModelPath,
    /// If `Some(ref)`, the design is applied under reference `ref` of the consuming
    /// model (i.e. `use design X for ref`). If `None`, the design is applied at the
    /// consuming model itself (i.e. `use design X`).
    pub applied_to: Option<ReferenceName>,
    /// Span of the `use design` declaration that created this application.
    pub span: Span,
}

/// A reference replacement in a design file (`use model as alias`).
///
/// The replacement path is stored here; submodel extractions from `with` clauses
/// are handled by the model's `SubmodelImport` entries which navigate through
/// the parent reference (allowing proper propagation of replacements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceReplacement {
    /// The path to the replacement model.
    pub replacement_path: ModelPath,
}

/// Resolved RHS for a single parameter assignment inside a design.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayParameterValue {
    /// Resolved parameter value (expression or piecewise).
    pub value: ParameterValue,
    /// Span of the design assignment (typically the design shorthand identifier).
    pub design_span: Span,
    /// Span of the full parameter definition on the target model’s IR (falls back to
    /// [`Self::design_span`] when the parameter is missing from the resolved target model).
    pub original_model_span: Span,
}

/// Declarative content of a `.one` design file.
///
/// A `Design` holds three kinds of contributions to a target model: overrides
/// of parameters that already exist on the target, additions of new parameters
/// that augment the target, and replacements of references with different
/// model files. Each kind has both a flat form (landing on the design's own
/// target) and a scoped form (landing on a descendant instance reachable via
/// an [`InstancePath`]). Scoped variants are populated when one design pulls
/// another in under a prefix via `use design Y for r` — `Y`'s top-level
/// contributions become scoped under `r` from the consumer's perspective.
///
/// Designs are merged composably (later wins) and instantiated by the
/// instancing pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Design {
    /// Model this design parameterizes (`design <name>`), when set.
    pub target_model: Option<ModelPath>,
    /// Overrides of parameters that already exist on the target model.
    pub parameter_overrides: IndexMap<ParameterName, OverlayParameterValue>,
    /// Overrides scoped under one or more reference segments from the target model
    /// (e.g. `x.ref = value`).
    pub scoped_overrides: IndexMap<InstancePath, IndexMap<ParameterName, OverlayParameterValue>>,
    /// Reference replacements (`use model as alias [with [submodels]]` in design files).
    /// Maps the reference alias to the replacement details.
    pub reference_replacements: IndexMap<ReferenceName, ReferenceReplacement>,
    /// Reference replacements scoped under an [`InstancePath`].
    ///
    /// Populated when an imported design's top-level reference replacements
    /// are folded in under a prefix (e.g. `use design Y for r` → `Y`'s
    /// replacements land under `r` from the consumer's perspective). The flat
    /// [`Self::reference_replacements`] map applies to the design's own
    /// target; this map carries the same shape but rooted somewhere deeper.
    pub scoped_replacements:
        IndexMap<InstancePath, IndexMap<ReferenceName, ReferenceReplacement>>,
    /// Parameters defined in the design that don't exist on the target model.
    /// These augment the target rather than overriding existing parameters.
    pub parameter_additions: IndexMap<ParameterName, Parameter>,
    /// Parameter additions scoped under an [`InstancePath`].
    ///
    /// Populated by the same prefix-folding mechanism as
    /// [`Self::scoped_replacements`]: an imported design's additions become
    /// scoped additions on the consumer when pulled in under `for r`.
    pub scoped_additions: IndexMap<InstancePath, IndexMap<ParameterName, Parameter>>,
}

impl Design {
    /// Creates an empty design with no declared target.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges `other` into `self` so that `other` wins on key collisions (later-wins stacking).
    ///
    /// The declared [`Self::target_model`] of `self` is not replaced by `other`.
    ///
    /// Both designs are assumed to target the same model — the resolver's
    /// `use design <Y>` handler enforces that invariant before calling. Under
    /// that assumption every kind of contribution (overrides, replacements,
    /// additions, and their scoped counterparts) is folded straight across by
    /// key, so that `other`'s overlays remain self-consistent: an override in
    /// `other` may reference a parameter `other` itself adds, and that
    /// addition stays available on the merged design.
    pub fn merge_later_wins(&mut self, other: &Self) {
        for (k, v) in &other.parameter_overrides {
            self.parameter_overrides.insert(k.clone(), v.clone());
        }
        merge_scoped(&mut self.scoped_overrides, &other.scoped_overrides);
        for (k, v) in &other.reference_replacements {
            self.reference_replacements.insert(k.clone(), v.clone());
        }
        merge_scoped(&mut self.scoped_replacements, &other.scoped_replacements);
        for (k, v) in &other.parameter_additions {
            self.parameter_additions.insert(k.clone(), v.clone());
        }
        merge_scoped(&mut self.scoped_additions, &other.scoped_additions);
    }

    /// Folds `other` into `self` under `prefix` — used when an outer design
    /// pulls another design in under a reference (`use design Y for r`).
    ///
    /// `prefix` is the navigation path *from `self`'s target down to where
    /// `other`'s target lives*. Conceptually `other`'s top-level contributions
    /// are about "things at `prefix`" from the consumer's vantage point, so
    /// they become scoped accordingly:
    ///
    /// - `other.parameter_overrides[p]`     → `self.scoped_overrides[prefix][p]`
    /// - `other.scoped_overrides[s][p]`     → `self.scoped_overrides[prefix.join(s)][p]`
    /// - `other.reference_replacements[a]`  → `self.scoped_replacements[prefix][a]`
    /// - `other.scoped_replacements[s][a]`  → `self.scoped_replacements[prefix.join(s)][a]`
    /// - `other.parameter_additions[p]`     → `self.scoped_additions[prefix][p]`
    /// - `other.scoped_additions[s][p]`     → `self.scoped_additions[prefix.join(s)][p]`
    ///
    /// A `prefix.is_root()` argument is treated like
    /// [`Self::merge_later_wins`].
    pub fn merge_prefixed(&mut self, prefix: &InstancePath, other: &Self) {
        if prefix.is_root() {
            self.merge_later_wins(other);
            return;
        }

        push_under_prefix(
            &mut self.scoped_overrides,
            prefix,
            &other.parameter_overrides,
        );
        for (path, m) in &other.scoped_overrides {
            let combined = prefix.join(path);
            let dest = self.scoped_overrides.entry(combined).or_default();
            for (k, v) in m {
                dest.insert(k.clone(), v.clone());
            }
        }

        push_under_prefix(
            &mut self.scoped_replacements,
            prefix,
            &other.reference_replacements,
        );
        for (path, m) in &other.scoped_replacements {
            let combined = prefix.join(path);
            let dest = self.scoped_replacements.entry(combined).or_default();
            for (k, v) in m {
                dest.insert(k.clone(), v.clone());
            }
        }

        push_under_prefix(
            &mut self.scoped_additions,
            prefix,
            &other.parameter_additions,
        );
        for (path, m) in &other.scoped_additions {
            let combined = prefix.join(path);
            let dest = self.scoped_additions.entry(combined).or_default();
            for (k, v) in m {
                dest.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Inserts every `(k, v)` from `entries` into `target[prefix]`, creating the
/// inner map on first use. Existing keys at `target[prefix]` are overwritten
/// (later-wins).
fn push_under_prefix<K, V>(
    target: &mut IndexMap<InstancePath, IndexMap<K, V>>,
    prefix: &InstancePath,
    entries: &IndexMap<K, V>,
) where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    if entries.is_empty() {
        return;
    }
    let dest = target.entry(prefix.clone()).or_default();
    for (k, v) in entries {
        dest.insert(k.clone(), v.clone());
    }
}

/// Merges `other` into `target` per-path, later-wins on inner keys.
fn merge_scoped<K, V>(
    target: &mut IndexMap<InstancePath, IndexMap<K, V>>,
    other: &IndexMap<InstancePath, IndexMap<K, V>>,
) where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    for (path, m) in other {
        let dest = target.entry(path.clone()).or_default();
        for (k, v) in m {
            dest.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oneil_shared::paths::ModelPath;
    use std::path::PathBuf;

    fn rep(path: &str) -> ReferenceReplacement {
        ReferenceReplacement {
            replacement_path: ModelPath::from_path_with_ext(&PathBuf::from(path)),
        }
    }

    #[test]
    fn merge_prefixed_scopes_reference_replacements_under_prefix() {
        let mut consumer = Design::new();
        let mut imported = Design::new();
        imported
            .reference_replacements
            .insert(ReferenceName::from("planet"), rep("/tmp/mars.on"));

        let prefix = InstancePath::root().child(ReferenceName::from("r"));
        consumer.merge_prefixed(&prefix, &imported);

        // The flat top-level replacements stay empty: replacing `planet` on the
        // outer model would target a non-existent reference.
        assert!(consumer.reference_replacements.is_empty());
        // The replacement is scoped under the prefix, where it actually lives
        // from the consumer's vantage point.
        let scoped = consumer
            .scoped_replacements
            .get(&prefix)
            .expect("prefix bucket present");
        assert_eq!(scoped.len(), 1);
        assert!(scoped.contains_key(&ReferenceName::from("planet")));
    }
}
