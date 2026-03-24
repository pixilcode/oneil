//! Resolution context and external context abstractions.

mod external;
mod resolution;

pub use external::{
    AstLoadingFailedError, ExternalResolutionContext, PythonImportLoadingFailedError,
};
pub use resolution::{
    MAX_BEST_MATCH_DISTANCE, ModelResolutionResult, ModelResult, ParameterResult,
    ReferencePathResult, ResolutionContext,
};
