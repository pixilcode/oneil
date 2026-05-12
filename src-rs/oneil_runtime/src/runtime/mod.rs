//! Runtime for the Oneil programming language.
//!
//! The runtime is split into submodules by concern: source loading, AST, IR,
//! evaluation, analysis, builtins, utilities, and (optionally) Python.

#![allow(
    clippy::multiple_inherent_impl,
    reason = "this allows the runtime to be split up into its different functionionalities"
)]

mod analysis;
mod ast;
mod builtin;
mod error;
mod eval;
mod ir;
mod source;
mod util;

#[cfg(feature = "python")]
mod python;

#[cfg(feature = "python")]
use std::path::PathBuf;

use crate::cache::{AstCache, EvalCache, IrCache, SourceCache};
#[cfg(feature = "python")]
use crate::cache::{PythonCallCache, PythonImportCache};
use oneil_builtins::BuiltinRef;

/// Runtime for the Oneil programming language.
///
/// The runtime manages caches for source files, ASTs, and IR, and provides
/// methods to load and process Oneil models.
#[derive(Debug)]
pub struct Runtime {
    source_cache: SourceCache,
    ast_cache: AstCache,
    ir_cache: IrCache,
    eval_cache: EvalCache,
    builtins: BuiltinRef,
    #[cfg(feature = "python")]
    py_features: PyFeatures,
}

/// Python features for the runtime
#[cfg(feature = "python")]
#[derive(Debug)]
pub struct PyFeatures {
    cache_dir: PathBuf,
    python_import_cache: PythonImportCache,
    python_call_cache: PythonCallCache,
    python_call_replacement_cache: PythonCallCache,
    cache_strategy: PythonCacheStrategy,
    cache_read_strategy: PythonCacheReadStrategy,
}

/// An enum representing the strategy that should be used
/// for python calls
#[cfg(feature = "python")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum PythonCacheStrategy {
    /// Always cache python calls
    AlwaysCache,
    /// Never cache python calls
    #[default]
    NeverCache,
}

/// An enum representing the strategy that should be used
/// for reading from the python call cache
#[cfg(feature = "python")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum PythonCacheReadStrategy {
    /// Always try to read from the cache
    #[default]
    AlwaysRead,
    /// Never read from the cache
    NeverRead,
}
