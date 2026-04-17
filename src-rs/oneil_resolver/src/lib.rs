#![cfg_attr(doc, doc = include_str!("../README.md"))]
//! Model resolver for the Oneil programming language

use indexmap::IndexMap;
use oneil_ast as ast;
use oneil_ir as ir;
use oneil_shared::paths::ModelPath;

mod context;
pub mod error;
mod resolver;
mod stack;

#[cfg(test)]
mod test;

pub use crate::context::{
    AstLoadingFailedError, ExternalResolutionContext, ModelResolutionResult,
    PythonImportLoadingFailedError,
};
pub use crate::error::{CircularDependencyError, DesignResolutionError, ResolutionErrorCollection};

use crate::context::ResolutionContext;

/// Result of loading one or more models: resolved models and any per-model errors.
#[derive(Debug)]
pub struct LoadModelResult {
    /// Resolved models by path.
    pub models: IndexMap<ModelPath, ir::Model>,
    /// Per-model resolution errors (import, reference, submodel, parameter, test, circular dependency).
    pub model_errors: IndexMap<ModelPath, ResolutionErrorCollection>,
}

/// Loads a single model and all its dependencies.
///
/// Returns the resolved models, per-model resolution errors, and circular dependency errors.
pub fn load_model<E>(
    model_path: &ModelPath,
    external_context: &mut E,
) -> IndexMap<ModelPath, ModelResolutionResult>
where
    E: ExternalResolutionContext,
{
    load_model_list(&[model_path], external_context)
}

/// Loads multiple models and all their dependencies.
///
/// Returns the resolved models, per-model resolution errors, and circular dependency errors.
pub fn load_model_list<E>(
    model_paths: &[&ModelPath],
    external_context: &mut E,
) -> IndexMap<ModelPath, ModelResolutionResult>
where
    E: ExternalResolutionContext,
{
    let mut resolution_context = ResolutionContext::new(external_context);

    for model_path in model_paths {
        resolver::load_model(model_path, &mut resolution_context);
    }

    resolution_context.into_result()
}

/// Resolves an expression as if it were in the context
/// of the given model.
///
/// # Errors
///
/// Returns the errors that occurred during variable resolution.
pub fn resolve_expr_in_model<E>(
    expr_ast: &ast::ExprNode,
    model_path: &ModelPath,
    external_context: &mut E,
) -> Result<ir::Expr, Vec<error::VariableResolutionError>>
where
    E: ExternalResolutionContext,
{
    let mut resolution_context = ResolutionContext::with_preloaded_models(external_context);
    resolution_context.push_active_model(model_path);

    resolver::resolve_expr(expr_ast, &resolution_context)
}
