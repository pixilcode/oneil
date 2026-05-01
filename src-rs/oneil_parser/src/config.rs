/// Configuration for the Oneil parser.
use oneil_shared::paths::ModelPath;

/// Options that affect parsing of whole-model input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::partial_pub_fields,
    reason = "`allow_design_shorthand` is parser-internal state set on the input span by `parse_design_file_decls` after a successful `design <model>` line; callers should not set it"
)]
pub struct Config {
    /// When `true`, a top-level `design <model>` line is required (`.one` design bundles).
    ///
    /// Ordinary `.on` model files keep this `false`; encountering a `design` header there
    /// is an error. `.one` files set this to `true`, so a missing header is also an error.
    pub require_design_header: bool,
    /// Internal: enables design-body shorthand (`id(.<seg>)* = expr`) for declarations.
    ///
    /// Callers should leave this `false`; the parser sets it on the input span's `extra`
    /// once it has successfully parsed a `design <model>` header so the rest of the file
    /// can use shorthand parameter assignments.
    pub(crate) allow_design_shorthand: bool,
}

impl Config {
    /// Creates a configuration with default settings (model file semantics).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            require_design_header: false,
            allow_design_shorthand: false,
        }
    }

    /// Returns parser settings implied by a [`ModelPath`] (by file extension).
    #[must_use]
    pub fn for_model_path(path: &ModelPath) -> Self {
        Self {
            require_design_header: path.is_design_file(),
            allow_design_shorthand: false,
        }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    fn default() -> Self {
        Self::new()
    }
}
