//! Source loading for the runtime.

use oneil_shared::{error::OneilDiagnostic, paths::SourcePath};

use super::Runtime;
use crate::{cache::InsertSourceResult, error::SourceError};

impl Runtime {
    /// Loads source code from a file, invalidating derived caches when the
    /// content has changed.
    ///
    /// This is the external entry point for callers that know a file may have
    /// changed on disk (LSP `didSave`, CLI). For internal use during
    /// resolution, call [`load_source_internal`](Self::load_source_internal)
    /// to avoid spurious mid-flight cache clears.
    ///
    /// # Errors
    ///
    /// Returns an [`OneilDiagnostic`] if the file could not be read.
    ///
    /// # Panics
    ///
    /// Panics if an internal cache invariant is violated (the entry was not
    /// found immediately after insertion).
    pub fn load_source(&mut self, path: &SourcePath) -> Result<&str, Box<OneilDiagnostic>> {
        let insert_result = self.insert_source(path);
        if matches!(
            insert_result,
            InsertSourceResult::InsertedNewSource | InsertSourceResult::UpdatedExistingSource
        ) {
            self.clear_non_source_caches(path);
        }
        self.source_cache
            .get_entry(path)
            .expect("it was just inserted")
            .map_err(|e| Box::new(OneilDiagnostic::from_error(e, path.clone().into_path_buf())))
    }

    /// Reads `path` from disk and caches the result without touching any
    /// derived caches (AST, unit graphs, eval).
    ///
    /// Used internally during resolution so that encountering a new dependency
    /// file does not wipe sibling caches mid-flight. Logs a warning if an
    /// already-cached file has changed on disk since it was last loaded, which
    /// means derived caches are stale; the caller should have gone through
    /// [`load_source`](Self::load_source) before starting the resolution pass.
    #[expect(
        clippy::print_stderr,
        reason = "intentional runtime warning for stale cache detection"
    )]
    pub(super) fn load_source_internal(&mut self, path: &SourcePath) -> Result<&str, &SourceError> {
        let insert_result = self.insert_source(path);
        if insert_result == InsertSourceResult::UpdatedExistingSource {
            eprintln!(
                "warning: {} changed on disk during resolution; \
                 derived caches may be stale. Call load_source before starting a resolution pass.",
                path.as_path().display(),
            );
        }
        self.source_cache
            .get_entry(path)
            .expect("it was just inserted")
    }

    /// Reads `path` from disk and inserts the result into the source cache.
    /// Returns whether the content is new or changed.
    fn insert_source(&mut self, path: &SourcePath) -> InsertSourceResult {
        let result = match std::fs::read_to_string(path.as_path()) {
            Ok(source) => Ok(source),
            Err(e) => Err(SourceError::new(path.clone(), e)),
        };
        self.source_cache.insert(path.clone(), result)
    }
}
