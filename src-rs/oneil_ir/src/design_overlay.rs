//! Designs and their declarative content.
//!
//! A [`Design`] is the resolved, declarative form of a `.one` design file:
//! parameter overrides for an existing target and parameter additions that
//! introduce new parameters. Designs are merged and consumed by the instancing
//! pass that wires up the live model tree.

use indexmap::IndexMap;
use oneil_shared::{InstancePath, paths::{DesignPath, ModelPath}, span::Span, symbols::ParameterName};

use crate::{Parameter, ParameterValue};

/// A design applied to a specific reference path on a model file.
///
/// `ApplyDesign` is the *declarative* record of an `apply <file> to <path>`
/// declaration. The actual stamping of the design's overrides and additions
/// onto the live tree happens in the instancing pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyDesign {
    /// Path to the `.one` design file applied here.
    pub design_path: DesignPath,
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
/// design's own target) and a scoped form (landing on a single named descendant
/// instance, e.g. `x.ref = value` in the design file).
///
/// Designs are composed and instantiated by the instancing pass. Nested
/// `apply X to ref` declarations within a design file are recorded separately
/// on the consuming model's IR and processed recursively during graph build.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Design {
    /// Model this design parameterizes (`design <name>`), when set.
    pub target_model: Option<ModelPath>,
    /// Overrides of parameters that already exist on the target model.
    pub parameter_overrides: IndexMap<ParameterName, OverlayParameterValue>,
    /// Overrides scoped under a single reference name from the target model
    /// (e.g. `x.ref = value`).
    pub scoped_overrides: IndexMap<InstancePath, IndexMap<ParameterName, OverlayParameterValue>>,
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
}
