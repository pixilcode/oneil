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
///   `submodel_path` contains the chain of *aliases* navigated within the
///   parent (e.g., `[x]`)
///
/// The map of submodels on a [`crate::Model`] is keyed by the submodel's
/// alias (a [`ReferenceName`]), because the alias is what determines instance
/// identity — `use foo as a` and `use foo as b` are two distinct instances
/// that can be replaced or overlaid independently. The [`name`](Self::name)
/// field on this struct holds the source-level model name (the `foo` in
/// `use foo as bar`) for diagnostics and the runtime API; it is *not* the
/// map key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmodelImport {
    /// Source-level model name as written in the file.
    ///
    /// For `use foo as bar` this is `foo`; for `use foo` this is `foo`. For
    /// `with`-extracted submodels (`use sat as s with [grav]`), this is the
    /// terminal identifier of the navigation (`grav`). The map key on the
    /// owning model is the alias ([`ReferenceName`]), not this name.
    name: SubmodelName,
    name_span: Span,
    /// The name of the reference that this submodel is associated with.
    ///
    /// For a direct submodel this equals the alias (which is also the map
    /// key). For an extracted submodel this is the alias under which the
    /// extraction was registered (also the map key) — the *parent*
    /// reference is recorded separately by the resolver via the parent's
    /// own [`ReferenceImport`].
    reference_name: ReferenceName,
    /// Chain of reference-name aliases navigated within the parent for
    /// extracted submodels. Empty for direct submodel imports.
    ///
    /// E.g., `use A as a with atmosphere.temperature as temp` resolves to
    /// `submodel_path = [atmosphere, temperature]` — each segment is the
    /// alias used at that level of nesting. Eval-time navigation walks the
    /// live reference graph using these names so each step picks up any
    /// per-instance reference replacements that may have been applied.
    submodel_path: Vec<ReferenceName>,
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
        submodel_path: Vec<ReferenceName>,
    ) -> Self {
        Self {
            name,
            name_span,
            reference_name,
            submodel_path,
        }
    }

    /// Returns the source-level model name of the submodel.
    ///
    /// See the struct-level documentation for what "source-level" means here.
    /// This is *not* the key used to look the submodel up on its owning model.
    #[must_use]
    pub const fn name(&self) -> &SubmodelName {
        &self.name
    }

    /// Returns the span of the name of the submodel.
    #[must_use]
    pub const fn name_span(&self) -> &Span {
        &self.name_span
    }

    /// Returns the reference name (= map key) the submodel is associated with.
    #[must_use]
    pub const fn reference_name(&self) -> &ReferenceName {
        &self.reference_name
    }

    /// Returns the chain of alias references navigated within the parent.
    /// Empty for direct submodel imports, non-empty for `with` extractions.
    #[must_use]
    pub fn submodel_path(&self) -> &[ReferenceName] {
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
