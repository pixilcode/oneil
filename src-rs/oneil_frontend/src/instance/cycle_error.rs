//! Errors emitted by the per-unit instance-graph build when the
//! [`CompilationUnit`] dependency graph contains a cycle.
//!
//! Although phrased as a "compilation" error, this diagnostic is
//! produced at *graph build* time, not by the resolver: the resolver
//! is intentionally cycle-agnostic (its active-model set is purely a
//! recursion guard) so that the cyclic references are visible in the
//! per-file templates by the time the per-unit build assembles them
//! together. See
//! `docs/decisions/2026-04-24-two-pass-instance-graph.md` for the
//! rationale.

use std::fmt;

use oneil_shared::{
    error::{AsOneilDiagnostic, Context, DiagnosticKind, ErrorLocation},
    paths::ModelPath,
    span::Span,
};

use super::compilation_unit::CompilationUnit;

/// A cycle in the compilation-unit dependency graph.
///
/// Emitted by the per-unit build when an attempt to recurse into a
/// child unit finds that the child is already on the build stack.
///
/// One error is emitted per **detection**, attributed to the cycle's
/// **target** — the file the cycle closes back onto, which is the
/// natural "root of the problem" from this build's perspective. For a
/// chain `A → B → C → B`, the target is `B`.
/// The recorded [`Self::span`] is the target file's own outgoing
/// reference declaration (the one pointing at the next unit on the
/// chain), so the diagnostic always renders inside the target file.
/// Other participants in the chain naturally surface the same cycle
/// from their own perspective when *they* are reached as the build
/// root via the cache (LSP open-file flows, CLI invocations against a
/// specific file).
///
/// [`Self::cycle`] lists the units in entry order, starting at the
/// target and ending at it again. For example, `A → B → C → B` is
/// recorded as `cycle = [B, C, B]`; the leading `A` is dropped because
/// it isn't part of the closed cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationCycleError {
    cycle: Vec<CompilationUnit>,
    source_path: ModelPath,
    span: Span,
}

impl CompilationCycleError {
    /// Creates a new cycle error with the given cycle path, the file
    /// the span belongs to, and the span of the outgoing reference
    /// declaration in that file.
    #[must_use]
    pub fn new(cycle: Vec<CompilationUnit>, source_path: ModelPath, span: Span) -> Self {
        debug_assert!(
            cycle.len() >= 2,
            "cycle path must include at least the back edge"
        );
        debug_assert_eq!(
            cycle.first(),
            cycle.last(),
            "cycle path must close on itself"
        );
        Self {
            cycle,
            source_path,
            span,
        }
    }

    /// Returns the cycle path in entry order, with the re-entered unit
    /// at both ends (so `cycle.first() == cycle.last()`).
    #[must_use]
    pub fn cycle(&self) -> &[CompilationUnit] {
        &self.cycle
    }

    /// Returns the file the [`Self::span`] points into. The runtime
    /// collector buckets errors by this path so a query for any
    /// participating file surfaces its own outgoing-reference span.
    #[must_use]
    pub const fn source_path(&self) -> &ModelPath {
        &self.source_path
    }

    /// Returns the back-edge span: the location of the reference (or
    /// apply) declaration in [`Self::source_path`] that participates
    /// in the cycle.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    fn render_chain(&self) -> String {
        self.cycle
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

impl fmt::Display for CompilationCycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "compilation cycle detected: {}", self.render_chain())
    }
}

impl AsOneilDiagnostic for CompilationCycleError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn diagnostic_location(&self, _source: &str) -> Option<ErrorLocation> {
        Some(ErrorLocation::from_span(&self.span))
    }

    fn context(&self) -> Vec<Context> {
        vec![Context::Help(
            "break the cycle by removing one of the references along the chain".to_string(),
        )]
    }
}
