//! Generic path-keyed cache using [`LoadResult`], and a source cache for raw file contents.

use std::hash::{DefaultHasher, Hash, Hasher};

use indexmap::IndexMap;
use oneil_parser::error::ParserError;
use oneil_resolver as resolver;
#[cfg(feature = "python")]
use oneil_shared::paths::PythonPath;
use oneil_shared::{
    load_result::LoadResult,
    paths::{ModelPath, SourcePath},
};

use crate::{error::SourceError, output};

#[cfg(feature = "python")]
use crate::error::PythonImportError;

/// Content hash for cached source, used to detect when file contents change.
pub fn source_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Result of inserting a source into the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertSourceResult {
    /// The source was inserted as a new entry.
    InsertedNewSource,
    /// A source with the same hash already exists in the cache.
    MatchingSourceExists,
}

/// Cached source for a path, with an optional content hash when load succeeded.
#[derive(Debug)]
struct SourceCacheEntry {
    /// Hash of the source when load succeeded; `None` when load failed.
    pub hash: u64,
    /// The loaded source or the load error.
    pub source: String,
}

/// Cache for source file contents keyed by path.
///
/// Stores a [`Result`] per path: either the file contents as a [`SourceCacheEntry`] or a
/// [`SourceError`](crate::error::SourceError) when loading failed.
///
/// This is specialized for source files because, unlike other caches,
/// there is no possible partial result.
#[derive(Debug)]
pub struct SourceCache {
    entries: IndexMap<SourcePath, Result<SourceCacheEntry, SourceError>>,
}

impl Default for SourceCache {
    fn default() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }
}

impl SourceCache {
    /// Creates an empty source cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached result for `path`, if present.
    #[must_use]
    pub fn get_entry(&self, path: &SourcePath) -> Option<Result<&str, &SourceError>> {
        self.entries
            .get(path)
            .map(|result| result.as_ref().map(|entry| entry.source.as_str()))
    }

    /// Inserts a result for `path`, replacing any existing entry. Computes and stores the content
    /// hash when the load succeeded.
    pub fn insert(
        &mut self,
        path: SourcePath,
        result: Result<String, SourceError>,
    ) -> InsertSourceResult {
        match result {
            Ok(source) => {
                // if the result is a source, compute the hash and check if it already exists
                let hash = source_hash(source.as_str());

                if self.contains_matching(&path, hash) {
                    InsertSourceResult::MatchingSourceExists
                } else {
                    let result = Ok(SourceCacheEntry { hash, source });
                    self.entries.insert(path, result);
                    InsertSourceResult::InsertedNewSource
                }
            }
            Err(e) => {
                self.entries.insert(path, Err(e));
                InsertSourceResult::InsertedNewSource
            }
        }
    }

    /// Checks if the cache contains an entry for `path` matching `source`. Uses hashes to determine
    /// equality.
    #[must_use]
    fn contains_matching(&self, path: &SourcePath, hash: u64) -> bool {
        self.entries
            .get(path)
            .is_some_and(|result| result.as_ref().is_ok_and(|entry| entry.hash == hash))
    }

    /// Returns an iterator over path–result pairs.
    pub fn paths(&self) -> impl Iterator<Item = &SourcePath> {
        self.entries.iter().map(|(path, _)| path)
    }
}

/// Cache for parsed AST models keyed by path.
pub type AstCache = ModelCache<output::ast::ModelNode, Vec<ParserError>>;

/// Cache for resolved IR models keyed by path.
pub type IrCache = ModelCache<output::ir::Model, resolver::ResolutionErrorCollection>;

/// Cache for evaluated output models keyed by path.
pub type EvalCache = ModelCache<output::Model, output::ModelEvalErrors>;

/// Cache for Python import function maps keyed by path.
///
/// Stores a [`Result`] per path: either the loaded [`PythonFunctionMap`] or a
/// [`PythonImportError`](crate::error::PythonImportError) when loading failed.
#[cfg(feature = "python")]
#[derive(Debug)]
pub struct PythonImportCache {
    entries: IndexMap<PythonPath, Result<oneil_python::function::PythonModule, PythonImportError>>,
}

#[cfg(feature = "python")]
impl Default for PythonImportCache {
    fn default() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }
}

#[cfg(feature = "python")]
impl PythonImportCache {
    /// Creates an empty Python import cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the full cached entry for `path`.
    #[must_use]
    pub fn get_entry(
        &self,
        path: &PythonPath,
    ) -> Option<&Result<oneil_python::function::PythonModule, PythonImportError>> {
        self.entries.get(path)
    }

    /// Inserts a result for `path`, replacing any existing entry.
    pub fn insert(
        &mut self,
        path: PythonPath,
        result: Result<oneil_python::function::PythonModule, PythonImportError>,
    ) {
        self.entries.insert(path, result);
    }

    /// Removes the cached entry for `path`, if present.
    pub fn remove(&mut self, path: &PythonPath) {
        self.entries.swap_remove(path);
    }
}

/// Generic cache keyed by path, storing [`LoadResult<T, E>`] per path.
///
/// Used to cache load outcomes (success, partial, or failure) for files or
/// resources identified by path.
#[derive(Debug)]
pub struct ModelCache<T, E> {
    entries: IndexMap<ModelPath, LoadResult<T, E>>,
}

impl<T, E> Default for ModelCache<T, E> {
    fn default() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }
}

impl<T, E> ModelCache<T, E> {
    /// Creates an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the full cached entry for `path`.
    #[must_use]
    pub fn get_entry(&self, path: &ModelPath) -> Option<&LoadResult<T, E>> {
        self.entries.get(path)
    }

    /// Returns the value for `path`, if present.
    #[must_use]
    pub fn get_value(&self, path: &ModelPath) -> Option<&T> {
        self.entries.get(path).and_then(LoadResult::value)
    }

    /// Returns the error for `path`, if present.
    #[must_use]
    pub fn get_error(&self, path: &ModelPath) -> Option<&E> {
        self.entries.get(path).and_then(LoadResult::error)
    }

    /// Inserts a [`LoadResult`] for `path`, replacing any existing entry.
    pub fn insert(&mut self, path: ModelPath, result: LoadResult<T, E>) {
        self.entries.insert(path, result);
    }

    /// Removes the cached entry for `path`, if present.
    pub fn remove(&mut self, path: &ModelPath) {
        self.entries.swap_remove(path);
    }

    /// Returns whether `path` has a cached entry.
    #[must_use]
    pub fn contains(&self, path: &ModelPath) -> bool {
        self.entries.contains_key(path)
    }

    /// Returns the number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over path–result pairs.
    pub fn iter(&self) -> indexmap::map::Iter<'_, ModelPath, LoadResult<T, E>> {
        self.entries.iter()
    }
}
