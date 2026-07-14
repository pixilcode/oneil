//! Type for the result of independent-parameter analysis.

use indexmap::IndexMap;
use oneil_output::Value;
use oneil_shared::{paths::ModelPath, symbols::ParameterName};

/// Map of model path to independent parameter names and their evaluated values.
///
/// A parameter is independent if it has no parameter or external dependencies.
#[derive(Debug, Clone, Default)]
pub struct Independents {
    /// Model path → parameter name → value.
    inner: IndexMap<ModelPath, IndexMap<ParameterName, Value>>,
}

impl Independents {
    /// Returns independents with no model entries.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Inserts a model path and its independent parameters into the map.
    pub fn insert(&mut self, path: ModelPath, params: IndexMap<ParameterName, Value>) {
        self.inner.insert(path, params);
    }

    /// Returns the independent parameters for a model path, if any.
    #[must_use]
    pub fn get(&self, path: &ModelPath) -> Option<&IndexMap<ParameterName, Value>> {
        self.inner.get(path)
    }

    /// Returns whether no independents were found for any model.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Merges another `Independents` into this one.
    ///
    /// Entries from `other` are inserted; existing keys are not overwritten
    /// (insertion order is preserved per `IndexMap`).
    pub fn extend(&mut self, other: Self) {
        self.inner.extend(other.inner);
    }

    /// Returns an iterator over (model path, independent name → value) entries.
    pub fn iter(&self) -> impl Iterator<Item = (&ModelPath, &IndexMap<ParameterName, Value>)> {
        self.inner.iter()
    }
}
