//! Source loading for the runtime.

use oneil_shared::{error::OneilDiagnostic, paths::SourcePath};

use super::Runtime;
use crate::{cache::InsertSourceResult, error::SourceError};

impl Runtime {
    /// Loads source code from a file.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeErrors`] (via [`get_model_errors`](super::Runtime::get_model_errors)) if the file could not be read.
    pub fn load_source(&mut self, path: &SourcePath) -> Result<&str, Box<OneilDiagnostic>> {
        self.load_source_internal(path)
            .map_err(|e| Box::new(OneilDiagnostic::from_error(e, path.clone().into_path_buf())))
    }

    pub(super) fn load_source_internal(&mut self, path: &SourcePath) -> Result<&str, &SourceError> {
        let result = match std::fs::read_to_string(path.as_path()) {
            Ok(source) => Ok(source),
            Err(e) => Err(SourceError::new(path.clone(), e)),
        };

        let insert_result = self.source_cache.insert(path.clone(), result);
        if insert_result == InsertSourceResult::InsertedNewSource {
            self.clear_non_source_caches(path);
        }

        self.source_cache
            .get_entry(path)
            .expect("it was just inserted")
    }
}
