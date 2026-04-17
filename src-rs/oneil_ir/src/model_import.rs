use oneil_shared::{
    paths::ModelPath,
    span::Span,
    symbols::{ReferenceName, SubmodelName},
};

/// An import for a submodel.
///
/// This can represent either:
/// - A direct submodel import (e.g., `use A as a`) where `submodel_path` is empty
/// - An extracted submodel via `with` clause (e.g., `use A as a with x`) where
///   `submodel_path` contains the path within the parent (e.g., `[x]`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmodelImport {
    name: SubmodelName,
    name_span: Span,
    /// The name of the reference that this submodel is associated with.
    reference_name: ReferenceName,
    /// Relative path within the parent reference for extracted submodels.
    /// Empty for direct submodel imports, non-empty for `with` extractions.
    /// E.g., `use A as a with atmosphere.temperature as temp` would have
    /// `submodel_path = [atmosphere, temperature]`.
    submodel_path: Vec<SubmodelName>,
}

impl SubmodelImport {
    /// Creates a new direct submodel import (no extraction path).
    #[must_use]
    pub const fn new(name: SubmodelName, name_span: Span, reference_name: ReferenceName) -> Self {
        Self {
            name,
            name_span,
            reference_name,
            submodel_path: Vec::new(),
        }
    }

    /// Creates a new extracted submodel import with a path within the parent.
    #[must_use]
    pub const fn extracted(
        name: SubmodelName,
        name_span: Span,
        reference_name: ReferenceName,
        submodel_path: Vec<SubmodelName>,
    ) -> Self {
        Self {
            name,
            name_span,
            reference_name,
            submodel_path,
        }
    }

    /// Returns the name of the submodel.
    #[must_use]
    pub const fn name(&self) -> &SubmodelName {
        &self.name
    }

    /// Returns the span of the name of the submodel.
    #[must_use]
    pub const fn name_span(&self) -> &Span {
        &self.name_span
    }

    /// Returns the reference name of the parent.
    #[must_use]
    pub const fn reference_name(&self) -> &ReferenceName {
        &self.reference_name
    }

    /// Returns the relative path within the parent reference.
    /// Empty for direct submodel imports, non-empty for `with` extractions.
    #[must_use]
    pub fn submodel_path(&self) -> &[SubmodelName] {
        &self.submodel_path
    }

    /// Returns true if this is an extracted submodel (via `with` clause).
    #[must_use]
    pub const fn is_extracted(&self) -> bool {
        !self.submodel_path.is_empty()
    }
}

/// An import for a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceImport {
    name: ReferenceName,
    name_span: Span,
    path: ModelPath,
}

impl ReferenceImport {
    /// Creates a new reference import with the given name and path.
    #[must_use]
    pub const fn new(name: ReferenceName, name_span: Span, path: ModelPath) -> Self {
        Self {
            name,
            name_span,
            path,
        }
    }

    /// Returns the name of the reference.
    #[must_use]
    pub const fn name(&self) -> &ReferenceName {
        &self.name
    }

    /// Returns the span of the name of the reference.
    #[must_use]
    pub const fn name_span(&self) -> &Span {
        &self.name_span
    }

    /// Returns the path of the reference.
    #[must_use]
    pub const fn path(&self) -> &ModelPath {
        &self.path
    }
}
