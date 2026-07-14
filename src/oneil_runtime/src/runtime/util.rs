//! Utility methods for the runtime.

use std::path::PathBuf;

use indexmap::IndexSet;
use oneil_shared::paths::PythonPath;
use oneil_shared::paths::{ModelPath, SourcePath};

use super::Runtime;
use crate::cache::{AstCache, EvalCache, IrCache, SourceCache};
use crate::{
    PythonCacheReadStrategy, PythonCacheStrategy,
    cache::{PythonCallCache, PythonImportCache},
    runtime::PyFeatures,
};
use oneil_builtins::BuiltinRef;

fn default_cache_dir() -> PathBuf {
    PathBuf::from("__oncache__")
}

impl Runtime {
    /// Creates a new runtime instance with empty caches.
    #[must_use]
    pub fn new() -> Self {
        let py_features = {
            let cache_dir = default_cache_dir();
            PyFeatures {
                python_import_cache: PythonImportCache::new(),
                python_call_cache: PythonCallCache::new(
                    cache_dir,
                    PythonCacheStrategy::default(),
                    PythonCacheReadStrategy::default(),
                ),
            }
        };

        Self {
            source_cache: SourceCache::new(),
            ast_cache: AstCache::new(),
            ir_cache: IrCache::new(),
            eval_cache: EvalCache::new(),
            builtins: BuiltinRef::new(),
            py_features,
        }
    }

    /// Creates a new runtime instance with empty caches that uses the
    /// provided caching strategies
    #[must_use]
    pub fn new_with_strategies(
        cache_strategy: PythonCacheStrategy,
        cache_read_strategy: PythonCacheReadStrategy,
    ) -> Self {
        let cache_dir = default_cache_dir();
        let py_features = PyFeatures {
            python_import_cache: PythonImportCache::new(),
            python_call_cache: PythonCallCache::new(cache_dir, cache_strategy, cache_read_strategy),
        };

        Self {
            source_cache: SourceCache::new(),
            ast_cache: AstCache::new(),
            ir_cache: IrCache::new(),
            eval_cache: EvalCache::new(),
            builtins: BuiltinRef::new(),
            py_features,
        }
    }

    /// Clears the runtime's caches for a given path.
    ///
    /// If the path is a model path (`.on`), clears the AST, IR, and eval caches for that path.
    /// If the path is a Python path (`.py`), clears the Python import cache for that path.
    ///
    /// This does not clear the source cache.
    pub fn clear_non_source_caches(&mut self, path: &SourcePath) {
        if let Ok(model_path) = ModelPath::try_from(path.clone()) {
            self.ast_cache.remove(&model_path);
            self.ir_cache.remove(&model_path);
            self.eval_cache.remove(&model_path);
        }

        if let Ok(python_path) = PythonPath::try_from(path.clone()) {
            self.py_features.python_import_cache.remove(&python_path);
        }
    }

    /// Gets the paths to files that the runtime relies on.
    #[must_use]
    pub fn get_watch_paths(&self) -> IndexSet<SourcePath> {
        self.source_cache.paths().cloned().collect()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
