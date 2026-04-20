//! Utility methods for the runtime.

use indexmap::IndexSet;
#[cfg(feature = "python")]
use oneil_shared::paths::PythonPath;
use oneil_shared::paths::{ModelPath, SourcePath};

use super::Runtime;
#[cfg(feature = "python")]
use crate::cache::PythonImportCache;
use crate::cache::{AstCache, EvalCache, IrCache, SourceCache};
use oneil_builtins::BuiltinRef;

impl Runtime {
    /// Creates a new runtime instance with empty caches.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source_cache: SourceCache::new(),
            ast_cache: AstCache::new(),
            ir_cache: IrCache::new(),
            eval_cache: EvalCache::new(),
            #[cfg(feature = "python")]
            python_import_cache: PythonImportCache::new(),
            builtins: BuiltinRef::new(),
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
        }
        // Any file change can invalidate transitive dependents, so the entire
        // eval cache must be cleared rather than just the changed path's entries.
        self.eval_cache.clear();

        #[cfg(feature = "python")]
        if let Ok(python_path) = PythonPath::try_from(path.clone()) {
            self.python_import_cache.remove(&python_path);
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
