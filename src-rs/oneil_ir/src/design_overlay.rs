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
/// model files. Designs are merged composably (later wins) and instantiated by
/// the instancing pass.
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
    /// Parameters defined in the design that don't exist on the target model.
    /// These augment the target rather than overriding existing parameters.
    pub parameter_additions: IndexMap<ParameterName, Parameter>,
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
    /// Note: `parameter_additions` are NOT merged. They are design-specific augmentations
    /// that should remain with the design file, not propagate to consumers.
    pub fn merge_later_wins(&mut self, other: &Self) {
        for (k, v) in &other.parameter_overrides {
            self.parameter_overrides.insert(k.clone(), v.clone());
        }
        for (path, m) in &other.scoped_overrides {
            let dest = self.scoped_overrides.entry(path.clone()).or_default();
            for (k, v) in m {
                dest.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in &other.reference_replacements {
            self.reference_replacements.insert(k.clone(), v.clone());
        }
        // parameter_additions intentionally NOT merged - see doc comment
    }

    /// Merges `other` into `self` under `prefix` (used when applying `use design … for <ref>`).
    ///
    /// Note: `parameter_additions` are NOT merged. They are design-specific augmentations
    /// that should remain with the design file, not propagate to consumers.
    pub fn merge_prefixed(&mut self, prefix: &InstancePath, other: &Self) {
        for (k, v) in &other.parameter_overrides {
            if prefix.is_root() {
                self.parameter_overrides.insert(k.clone(), v.clone());
            } else {
                self.scoped_overrides
                    .entry(prefix.clone())
                    .or_default()
                    .insert(k.clone(), v.clone());
            }
        }
        for (path, m) in &other.scoped_overrides {
            let combined = prefix.join(path);
            let dest = self.scoped_overrides.entry(combined).or_default();
            for (k, v) in m {
                dest.insert(k.clone(), v.clone());
            }
        }
        // Reference replacements apply directly (prefix doesn't affect them since
        // they replace by alias name, not by path)
        for (k, v) in &other.reference_replacements {
            self.reference_replacements.insert(k.clone(), v.clone());
        }
        // parameter_additions intentionally NOT merged - see doc comment
    }
}
