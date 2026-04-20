/// Configuration for the Oneil parser.
use oneil_shared::paths::ModelPath;

/// Options that affect parsing of whole-model input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// When `true`, a top-level `design <model>` line is required (`.one` design bundles).
    ///
    /// Ordinary `.on` model files keep this `false`; encountering a `design` header there
    /// is an error. `.one` files set this to `true`, so a missing header is also an error.
    pub require_design_header: bool,
}

impl Config {
    /// Creates a configuration with default settings (model file semantics).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            require_design_header: false,
        }
    }

    /// Returns parser settings implied by a [`ModelPath`] (by file extension).
    #[must_use]
    pub fn for_model_path(path: &ModelPath) -> Self {
        Self {
            require_design_header: path.is_design_file(),
        }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    fn default() -> Self {
        Self::new()
    }
}
