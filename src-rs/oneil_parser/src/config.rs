/// Configuration for the Oneil parser.
use oneil_shared::paths::ModelPath;

/// Options that affect parsing of whole-model input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// When `true`, a top-level `design <model>` line is accepted (`.one` design bundles).
    ///
    /// Ordinary `.on` model files keep this `false` so `design` is only available via `use design`.
    pub allow_design_header: bool,
}

impl Config {
    /// Creates a configuration with default settings (model file semantics).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            allow_design_header: false,
        }
    }

    /// Returns parser settings implied by a [`ModelPath`] (by file extension).
    #[must_use]
    pub fn for_model_path(path: &ModelPath) -> Self {
        Self {
            allow_design_header: path.is_design_bundle(),
        }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    fn default() -> Self {
        Self::new()
    }
}
