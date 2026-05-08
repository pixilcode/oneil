//! Testing for Oneil model IR.

use oneil_shared::{labels::SectionLabel, span::Span};

use crate::{Dependencies, debug_info::TraceLevel, expr::Expr};

/// A test within a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Test {
    span: Span,
    trace_level: TraceLevel,
    expr: Expr,
    dependencies: Dependencies,
    section_label: Option<SectionLabel>,
}

impl Test {
    /// Creates a new test with the specified properties.
    #[must_use]
    pub const fn new(
        span: Span,
        trace_level: TraceLevel,
        expr: Expr,
        dependencies: Dependencies,
        section_label: Option<SectionLabel>,
    ) -> Self {
        Self {
            span,
            trace_level,
            expr,
            dependencies,
            section_label,
        }
    }

    /// Returns the span of the entire test definition.
    #[must_use]
    pub const fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the trace level for this test.
    #[must_use]
    pub const fn trace_level(&self) -> TraceLevel {
        self.trace_level
    }

    /// Returns the test expression that defines the expected behavior.
    #[must_use]
    pub const fn expr(&self) -> &Expr {
        &self.expr
    }

    /// Returns a mutable reference to the test expression.
    pub const fn expr_mut(&mut self) -> &mut Expr {
        &mut self.expr
    }

    /// Returns the dependencies of this test.
    #[must_use]
    pub const fn dependencies(&self) -> &Dependencies {
        &self.dependencies
    }

    /// Returns the section label for this test, if any.
    #[must_use]
    pub const fn section_label(&self) -> Option<&SectionLabel> {
        self.section_label.as_ref()
    }
}
