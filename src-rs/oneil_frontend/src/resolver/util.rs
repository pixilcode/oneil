//! Declarations gathered from a model AST for downstream resolver passes.

use oneil_ast as ast;

/// A parameter node and the section header it lives under, if any.
///
/// Top-level parameters use `section_label: None`.
#[derive(Debug, Clone, Copy)]
pub struct ParameterWithSection<'a> {
    pub parameter: &'a ast::ParameterNode,
    pub section_label: Option<&'a ast::SectionLabelNode>,
}

/// A test node and the section header it lives under, if any.
///
/// Top-level tests use `section_label: None`.
#[derive(Debug, Clone, Copy)]
pub struct TestWithSection<'a> {
    pub test: &'a ast::TestNode,
    pub section_label: Option<&'a ast::SectionLabelNode>,
}
