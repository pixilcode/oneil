//! Design types used during the instance-graph build.
//!
//! [`ApplyDesign`] is the public declarative record of an `apply X to ref`
//! declaration.

use indexmap::IndexMap;
use oneil_ir as ir;
use oneil_shared::{
    InstancePath,
    paths::{DesignPath, ModelPath},
    span::Span,
    symbols::{ParameterName, TestIndex},
};

/// Declarative record of an `apply <file> to <path>` declaration.
///
/// Carried in [`ModelResolutionResult`](crate::ModelResolutionResult) for each
/// model that declares applies. The build pass consumes these records to apply
/// design contributions to the live instance tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyDesign {
    /// Path to the `.one` design file being applied.
    pub design_path: DesignPath,
    /// Reference-name path on the consuming model identifying the target instance.
    pub target: InstancePath,
    /// Span of the `apply` declaration that produced this record.
    pub span: Span,
}

/// Resolved RHS for a single parameter assignment inside a design.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OverlayParameterValue {
    /// Resolved parameter value (expression or piecewise).
    pub value: ir::ParameterValue,
    /// Span of the design assignment identifier.
    pub design_span: Span,
    /// Span of the full parameter definition on the target model (falls back to
    /// `design_span` when the target parameter is absent from the resolved model).
    pub original_model_span: Span,
}

/// Resolved content of a `.one` design file.
///
/// Holds parameter overrides for an existing target and parameter additions
/// that augment the target. Scoped overrides (`x.ref = value`) cover nested
/// reference paths directly declared in the design file. Nested `apply X to
/// ref` declarations within a design file are recorded separately and
/// processed recursively by the graph builder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Design {
    /// Model this design parameterizes (`design <name>`), when set.
    pub(crate) target_model: Option<ModelPath>,
    /// Overrides of parameters that already exist on the target model.
    pub(crate) parameter_overrides: IndexMap<ParameterName, OverlayParameterValue>,
    /// Overrides scoped under one or more reference segments from the target
    /// (e.g. `x.ref = value` in the design file).
    pub(crate) scoped_overrides:
        IndexMap<InstancePath, IndexMap<ParameterName, OverlayParameterValue>>,
    /// Parameters defined in the design that don't exist on the target model.
    pub(crate) parameter_additions: IndexMap<ParameterName, ir::Parameter>,
    /// Tests defined in the design that are added to the target model.
    /// Test expressions are evaluated in the target's scope.
    pub(crate) test_additions: IndexMap<TestIndex, ir::Test>,
}

impl Design {
    /// Creates an empty design with no declared target.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
