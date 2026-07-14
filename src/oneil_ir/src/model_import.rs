use oneil_shared::{
    paths::ModelPath,
    span::Span,
    symbols::{ReferenceName, SubmodelName},
};

/// An import for a submodel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmodelImport {
    name: SubmodelName,
    name_span: Span,
    /// the name of the reference that this submodel is associated with
    reference_name: ReferenceName,
}

impl SubmodelImport {
    /// Creates a new submodel import with the given name and path.
    #[must_use]
    pub const fn new(name: SubmodelName, name_span: Span, reference_name: ReferenceName) -> Self {
        Self {
            name,
            name_span,
            reference_name,
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

    /// Returns the path of the submodel.
    #[must_use]
    pub const fn reference_name(&self) -> &ReferenceName {
        &self.reference_name
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
