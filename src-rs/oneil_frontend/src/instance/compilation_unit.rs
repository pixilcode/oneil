//! [`CompilationUnit`]: the cache-key + cycle-stack identity for the
//! per-unit instance-graph build.
//!
//! A *compilation unit* is the smallest thing the per-unit build pass
//! ([`build_unit_graph`](super::graph::build_unit_graph)) produces a
//! cached [`InstanceGraph`](super::graph::InstanceGraph) for. Today there
//! are two kinds:
//!
//! - [`CompilationUnit::Model`]: a model file (`.on` - no `design <...>` declaration).
//!   The cached graph is the file rooted as the user-facing root, with the file's
//!   own `apply` declarations applied to children.
//!
//! - [`CompilationUnit::Design`]: a design file (`.one` with a
//!   `design <model>` declaration). The cached graph is the design's
//!   *target model* with the design's own contributions and `apply`
//!   declarations overlaid at the root anchor. Instance keys inside
//!   reference the target model's path, since the instances *are*
//!   target-model instances; the `Design(_)` cache key just records
//!   that this overlay flavour is the one stored.

use std::fmt;

use oneil_shared::paths::{DesignPath, ModelPath};

/// Cache-key identity for the per-unit graph build.
///
/// See the module docs for the kinds of units and their semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompilationUnit {
    /// A model file. The cached graph is the file rooted as itself.
    Model(ModelPath),
    /// A design file. Reserved for the design-unit caching follow-up.
    Design(DesignPath),
}

impl CompilationUnit {
    /// Returns the source-file path this unit lives in (`.on` for
    /// models, `.one` for designs), as a [`ModelPath`] for ergonomic
    /// comparison against the file maps in the runtime (which key
    /// design files by their lossless [`ModelPath`] form).
    #[must_use]
    pub fn source_path(&self) -> ModelPath {
        match self {
            Self::Model(p) => p.clone(),
            Self::Design(p) => p.to_model_path(),
        }
    }

    /// Returns `true` if this unit is a model file.
    #[must_use]
    pub const fn is_model(&self) -> bool {
        matches!(self, Self::Model(_))
    }

    /// Returns `true` if this unit is a design file.
    #[must_use]
    pub const fn is_design(&self) -> bool {
        matches!(self, Self::Design(_))
    }
}

impl fmt::Display for CompilationUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(p) => write!(f, "{}", p.as_path().display()),
            Self::Design(p) => write!(f, "{}", p.as_path().display()),
        }
    }
}

impl From<ModelPath> for CompilationUnit {
    fn from(value: ModelPath) -> Self {
        Self::Model(value)
    }
}

impl From<DesignPath> for CompilationUnit {
    fn from(value: DesignPath) -> Self {
        Self::Design(value)
    }
}
