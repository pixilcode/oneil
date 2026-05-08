/// Configuration for the Oneil parser.
use std::{path::Path, sync::Arc};

use oneil_shared::paths::ModelPath;

/// Options that affect parsing of whole-model input.
///
/// In addition to the boolean flags that control parser behaviour, `Config`
/// carries the file path and the full source text so that every [`Span`] that
/// the parser produces is self-contained (see [`oneil_shared::span::Span`]).
///
/// [`Span`]: oneil_shared::span::Span
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Path of the source file being parsed.  May be empty for in-memory / test inputs.
    pub(crate) path: Arc<Path>,
    /// Full source text of the file being parsed.
    pub(crate) source: Arc<str>,
}

impl Config {
    /// Creates a configuration with default settings (model file semantics) and no path/source.
    ///
    /// Useful for tests and one-off expression parsing where no file path is available.
    /// The resulting [`Span`]s will carry an empty path and empty source string.
    ///
    /// [`Span`]: oneil_shared::span::Span
    #[must_use]
    pub fn new() -> Self {
        Self {
            require_design_header: false,
            allow_design_shorthand: false,
            path: Arc::from(Path::new("")),
            source: Arc::from(""),
        }
    }

    /// Creates a configuration with the given source file path and full source text.
    ///
    /// Every [`Span`] produced while parsing will carry clones of these `Rc`s, so the
    /// path and source text are shared cheaply across the entire AST.
    ///
    /// [`Span`]: oneil_shared::span::Span
    #[must_use]
    pub fn with_source(path: Arc<Path>, source: Arc<str>) -> Self {
        Self {
            path,
            source,
            ..Self::new()
        }
    }

    /// Returns parser settings implied by a [`ModelPath`] and the full source text.
    #[must_use]
    pub fn for_model_path(model_path: &ModelPath, path: Arc<Path>, source: Arc<str>) -> Self {
        Self {
            require_design_header: model_path.is_design_file(),
            allow_design_shorthand: false,
            path,
            source,
        }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    fn default() -> Self {
        Self::new()
    }
}
