//! Generic path-keyed cache using [`LoadResult`], and a source cache for raw file contents.

use std::hash::{DefaultHasher, Hash, Hasher};

use indexmap::IndexMap;
use oneil_parser::error::ParserError;
use oneil_resolver as resolver;
use oneil_shared::{
    load_result::LoadResult,
    paths::{ModelPath, SourcePath},
};

use crate::{error::SourceError, output};

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

#[cfg(feature = "python")]
pub use python::{PythonCallCache, PythonCallCacheRecord, PythonImportCache};

#[cfg(feature = "python")]
mod python {
    use std::path::{Component, PathBuf};

    use indexmap::IndexMap;
    use oneil_py_call_cache::{
        CacheValue, CachedFunctionName, CachedParameterName, CachedTestIndex, FileCache,
        FunctionCall, FunctionCallResult, ImportHash, ReadCacheError, WriteCacheError,
    };
    use oneil_py_call_cache::{CachedPythonPath, ImportEntry};
    use oneil_python::{PythonEvalError, function::PythonModule};
    use oneil_shared::paths::ModelPath;
    use oneil_shared::{
        paths::PythonPath,
        symbols::{ParameterName, PyFunctionName, TestIndex},
    };

    use crate::error::PythonImportError;
    use crate::output;

    /// Inputs shared by [`PythonCallCache::add_parameter_entry`] and [`PythonCallCache::add_test_entry`].
    #[derive(Debug)]
    pub struct PythonCallCacheRecord<'a> {
        /// Model file whose cache entry is updated.
        pub model_path: &'a ModelPath,
        /// Path of the Python module that defined the callee.
        pub python_path: &'a PythonPath,
        /// Name of the invoked Python function.
        pub function_name: &'a PyFunctionName,
        /// Argument values passed to the call.
        pub args: &'a [output::Value],
        /// Evaluation outcome to persist.
        pub eval_result: Result<output::Value, PythonEvalError>,
        /// Loaded module metadata for import tracking.
        pub python_module: &'a PythonModule,
    }

    /// Whether a recorded call belongs to a parameter default or a test body.
    #[derive(Debug, Clone, Copy)]
    enum CallCacheTarget<'a> {
        Parameter(&'a ParameterName),
        Test(TestIndex),
    }
    /// Cache for Python import function maps keyed by path.
    ///
    /// Stores a [`Result`] per path: either the loaded [`PythonFunctionMap`] or a
    /// [`PythonImportError`](crate::error::PythonImportError) when loading failed.
    #[cfg(feature = "python")]
    #[derive(Debug)]
    pub struct PythonImportCache {
        entries:
            IndexMap<PythonPath, Result<oneil_python::function::PythonModule, PythonImportError>>,
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

    /// Cache for Python function calls keyed by path.
    #[cfg(feature = "python")]
    #[derive(Debug)]
    pub struct PythonCallCache {
        cache_dir: PathBuf,
        entries: IndexMap<ModelPath, FileCache>,
    }

    #[cfg(feature = "python")]
    impl PythonCallCache {
        /// Creates an empty Python call cache.
        #[must_use]
        pub fn new(cache_dir: PathBuf) -> Self {
            Self {
                cache_dir,
                entries: IndexMap::new(),
            }
        }

        /// Clears the cache.
        pub fn clear(&mut self) {
            self.entries.clear();
        }

        /// Merges another cache into this one.
        ///
        /// If there are conflicting entries, the entries in the other cache are preferred.
        pub fn merge(&mut self, other: Self) {
            self.entries.extend(other.entries);
        }

        /// Returns the cached entry for `parameter` in `model_path`, if present.
        ///
        /// If the cache entry has not been loaded yet, it is loaded from disk.
        ///
        /// # Errors
        ///
        /// Returns [`ReadCacheError`] if the cache file cannot be read.
        pub fn get_parameter_entry(
            &mut self,
            model_path: &ModelPath,
            parameter_name: &ParameterName,
        ) -> Option<&[FunctionCall]> {
            self.load(model_path).ok()?;

            let entry = self.entries.get(model_path)?;
            entry.parameters.get(parameter_name).map(Vec::as_slice)
        }

        pub fn get_test_entry(
            &mut self,
            model_path: &ModelPath,
            test_index: TestIndex,
        ) -> Option<&[FunctionCall]> {
            self.load(model_path).ok()?;

            let entry = self.entries.get(model_path)?;
            entry.tests.get(&test_index).map(Vec::as_slice)
        }

        /// Appends one cached function call for `parameter_name` and updates import usage.
        pub fn add_parameter_entry(
            &mut self,
            record: PythonCallCacheRecord<'_>,
            parameter_name: &ParameterName,
        ) {
            self.push_function_call_entry(record, CallCacheTarget::Parameter(parameter_name));
        }

        /// Appends one cached function call for `test_index` and updates import usage.
        pub fn add_test_entry(&mut self, record: PythonCallCacheRecord<'_>, test_index: TestIndex) {
            self.push_function_call_entry(record, CallCacheTarget::Test(test_index));
        }

        /// Appends one [`FunctionCall`] under `target` and registers `function_name` on the matching import entry.
        fn push_function_call_entry(
            &mut self,
            record: PythonCallCacheRecord<'_>,
            target: CallCacheTarget<'_>,
        ) {
            let PythonCallCacheRecord {
                model_path,
                python_path,
                function_name,
                args,
                eval_result,
                python_module,
            } = record;

            let model_entry = self.entries.entry(model_path.clone()).or_default();
            let cached_function_call = function_call_from(function_name, args, eval_result);

            match target {
                CallCacheTarget::Parameter(parameter_name) => {
                    let cached_parameter_name = CachedParameterName::from(parameter_name.clone());

                    model_entry
                        .parameters
                        .entry(cached_parameter_name)
                        .or_default()
                        .push(cached_function_call);
                }
                CallCacheTarget::Test(test_index) => {
                    let cached_test_index = CachedTestIndex::from(test_index);

                    model_entry
                        .tests
                        .entry(cached_test_index)
                        .or_default()
                        .push(cached_function_call);
                }
            }

            let cached_python_path = CachedPythonPath::from(python_path.clone());
            let cached_function_name = CachedFunctionName::from(function_name.clone());
            model_entry
                .imports
                .entry(cached_python_path)
                .or_insert_with(|| make_python_cache_import_entry(python_path, python_module))
                .functions_used
                .insert(cached_function_name);
        }

        /// Loads a cache entry from disk.
        ///
        /// If the cache entry already exists, this does nothing.
        ///
        /// # Errors
        ///
        /// Returns [`ReadCacheError`] if the cache file cannot be read.
        fn load(&mut self, model_path: &ModelPath) -> Result<(), ReadCacheError> {
            if self.entries.contains_key(model_path) {
                return Ok(());
            }

            let cache_relative_path = get_cache_relative_path(model_path);
            let cache_path = self.cache_dir.join(cache_relative_path);
            let cache = FileCache::read_from_path(cache_path)?;
            self.entries.insert(model_path.clone(), cache);
            Ok(())
        }

        /// Saves all cache entries to disk.
        ///
        /// # Errors
        ///
        /// Returns a vector of [`WriteCacheError`] if the cache files cannot be written.
        pub fn save_all(&self) -> Result<(), Vec<WriteCacheError>> {
            let mut errors = Vec::new();

            for (model_path, cache) in &self.entries {
                let cache_relative_path = get_cache_relative_path(model_path);
                let cache_path = self.cache_dir.join(cache_relative_path);

                // if the cache file doesn't exist and the cache is empty,
                // don't write it to disk
                if !cache_path.exists() && cache.is_empty() {
                    continue;
                }

                match cache.write_to_path(cache_path) {
                    Ok(()) => (),
                    Err(e) => errors.push(e),
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }

        /// Validates that the Python import for `model_path` is still valid by
        /// checking the hash of the Python module.
        ///
        /// If the hash does not match, the Python import is cleared from the
        /// cache and all cached function calls based on that import are cleared.
        pub fn validate_or_clear_python_import(
            &mut self,
            model_path: &ModelPath,
            python_path: &PythonPath,
            python_module_hash: u64,
        ) {
            // make sure the model path has been loaded, if it exists
            let Some(()) = self.load(model_path).ok() else {
                return;
            };

            //
            let Some(model_entry) = self.entries.get_mut(model_path) else {
                return;
            };

            // get the import entry for the Python path
            let Some(import_entry) = model_entry.imports.get(python_path) else {
                return;
            };

            // check if the import entry is still valid by comparing the hash
            if import_entry.hash == python_module_hash {
                return;
            }

            // remove the import entry from the model entry
            let Some(removed_import_entry) = model_entry.imports.remove(python_path) else {
                return;
            };

            // remove cached function calls based on the functions from the removed import entry
            let import_functions = removed_import_entry.functions_used;

            for calls in model_entry.parameters.values_mut() {
                calls.retain(|call| !import_functions.contains(&call.function));
            }

            for calls in model_entry.tests.values_mut() {
                calls.retain(|call| !import_functions.contains(&call.function));
            }
        }
    }

    fn get_cache_relative_path(model_path: &ModelPath) -> PathBuf {
        model_path
            .as_path()
            .with_extension("json")
            .components()
            // convert to a path that can be used in the cache directory
            .fold(PathBuf::new(), append_normalized_component)
    }

    fn append_normalized_component(mut path: PathBuf, component: Component<'_>) -> PathBuf {
        match component {
            Component::Prefix(_) => {
                path.push("__prefix__");
            }
            Component::RootDir => {
                path.push("__root__");
            }
            Component::CurDir => {}
            // in order to avoid overwriting files outside of the cache directory,
            // we convert ".." to "__parent__"
            Component::ParentDir => {
                if let Some(parent) = path.parent()
                    && !parent.ends_with("__parent__")
                {
                    path = parent.to_path_buf();
                } else {
                    path.push("__parent__");
                }
            }
            Component::Normal(os_str) => {
                path.push(os_str);
            }
        }

        path
    }

    fn function_call_from(
        function_name: &PyFunctionName,
        args: &[output::Value],
        eval_result: Result<output::Value, PythonEvalError>,
    ) -> FunctionCall {
        FunctionCall {
            function: CachedFunctionName::from(function_name.clone()),
            inputs: args
                .iter()
                .map(|arg| CacheValue::from(arg.clone()))
                .collect(),
            output: FunctionCallResult::from(eval_result),
        }
    }

    fn make_python_cache_import_entry(
        python_path: &PythonPath,
        python_module: &PythonModule,
    ) -> ImportEntry {
        let name = python_path
            .as_path()
            .file_stem()
            .map_or_else(|| "<unknown>".into(), |s| s.to_string_lossy().to_string());

        let dependencies = python_module
            .get_imports()
            .iter()
            .map(|path| path.display().to_string())
            .collect();

        let hash = ImportHash::from(python_module.get_hash());

        ImportEntry::new(name, dependencies, hash)
    }
}
