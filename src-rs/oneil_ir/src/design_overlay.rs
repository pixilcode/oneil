//! Designs and their declarative content.
//!
//! A [`Design`] is the resolved, declarative form of a `.one` design file:
//! parameter overrides for an existing target and parameter additions that
//! introduce new parameters. Designs are merged and consumed by the instancing
//! pass that wires up the live model tree.

use indexmap::IndexMap;
use oneil_shared::{InstancePath, paths::ModelPath, span::Span, symbols::ParameterName};

use crate::{Parameter, ParameterValue};

/// A design applied to a specific reference path on a model file.
///
/// `ApplyDesign` is the *declarative* record of an `apply <file> to <path>`
/// declaration. The actual stamping of the design's overrides and additions
/// onto the live tree happens in the instancing pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyDesign {
    /// Path to the `.one` design file applied here.
    pub design_path: ModelPath,
    /// Reference-name path on the consuming model identifying the instance the
    /// design lands on. Always non-empty: targetless applies are no longer a
    /// feature, and the path may have multiple segments (e.g. `sc.U`).
    pub target: InstancePath,
    /// Span of the `apply` declaration that produced this record.
    pub span: Span,
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
/// A `Design` holds two kinds of contributions to a target model: overrides of
/// parameters that already exist on the target and additions of new parameters
/// that augment the target. Each kind has both a flat form (landing on the
/// design's own target) and a scoped form (landing on a descendant instance
/// reachable via an [`InstancePath`]). Scoped variants are populated when a
/// design is applied under a non-root path via `apply X to a.b` — the design's
/// top-level contributions become scoped under `a.b` from the consumer's
/// perspective.
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
    /// Parameters defined in the design that don't exist on the target model.
    /// These augment the target rather than overriding existing parameters.
    pub parameter_additions: IndexMap<ParameterName, Parameter>,
    /// Parameter additions scoped under an [`InstancePath`].
    ///
    /// Populated by the same prefix-folding mechanism as
    /// [`Self::scoped_overrides`]: an imported design's additions become
    /// scoped additions on the consumer when pulled in under a non-root path.
    pub scoped_additions: IndexMap<InstancePath, IndexMap<ParameterName, Parameter>>,
}

impl Design {
    /// Creates an empty design with no declared target.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Folds `other` into `self` under `prefix`. `prefix` is the navigation
    /// path *from `self`'s target down to where `other`'s target lives*.
    /// Conceptually `other`'s top-level contributions are about "things at
    /// `prefix`" from the consumer's vantage point, so they become scoped
    /// accordingly:
    ///
    /// - `other.parameter_overrides[p]` → `self.scoped_overrides[prefix][p]`
    /// - `other.scoped_overrides[s][p]` → `self.scoped_overrides[prefix.join(s)][p]`
    /// - `other.parameter_additions[p]` → `self.scoped_additions[prefix][p]`
    /// - `other.scoped_additions[s][p]` → `self.scoped_additions[prefix.join(s)][p]`
    ///
    /// `prefix` must be non-root: design files no longer support targetless
    /// inheritance, so every cross-design merge happens under some path.
    pub fn merge_prefixed(&mut self, prefix: &InstancePath, other: &Self) {
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
