//! Evaluation instance path: reference aliases from the root model to a subtree.

use crate::{paths::ModelPath, symbols::ReferenceName};

/// Path from the evaluation root through successive `use` / `ref` aliases.
///
/// Two imports of the same on-disk model under different aliases correspond to
/// different instance paths, so evaluated parameter stores are keyed by
/// [`EvalInstanceKey`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct InstancePath(Vec<ReferenceName>);

impl InstancePath {
    /// The root model instance (no reference segments).
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    /// Returns the number of reference segments.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when this is empty (no segments).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` when this is the root instance.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.is_empty()
    }

    /// Returns the chain of reference names.
    #[must_use]
    pub fn segments(&self) -> &[ReferenceName] {
        &self.0
    }

    /// Pushes a reference segment, returning a new path.
    #[must_use]
    pub fn child(&self, edge: ReferenceName) -> Self {
        let mut v = self.0.clone();
        v.push(edge);
        Self(v)
    }

    /// If `longer` starts with this path's segments, returns the remaining suffix.
    ///
    /// Otherwise returns `None` (including when `longer` is shorter than `self`).
    #[must_use]
    pub fn strip_prefix_of(&self, longer: &Self) -> Option<Self> {
        let p = self.segments();
        let c = longer.segments();
        if c.get(..p.len()) != Some(p) {
            return None;
        }
        Some(Self(c[p.len()..].to_vec()))
    }

    /// Concatenates this path with an additional suffix.
    #[must_use]
    pub fn join(&self, suffix: &Self) -> Self {
        let mut v = self.0.clone();
        v.extend(suffix.segments().iter().cloned());
        Self(v)
    }
}

/// Uniquely identifies one evaluated occurrence of a model file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvalInstanceKey {
    /// On-disk model path.
    pub model_path: ModelPath,
    /// Which import chain led to this evaluation.
    pub instance_path: InstancePath,
}

impl EvalInstanceKey {
    /// Creates a key for the root instance of `model_path`.
    #[must_use]
    pub const fn root(model_path: ModelPath) -> Self {
        Self {
            model_path,
            instance_path: InstancePath::root(),
        }
    }
}
