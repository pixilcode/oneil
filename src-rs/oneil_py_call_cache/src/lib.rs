#![cfg_attr(doc, doc = include_str!("../README.md"))]
//! Caching python function calls for the Oneil programming language.

mod error;
mod file;
mod function_call;
mod identifiers;
mod value;

pub use error::{ReadCacheError, WriteCacheError};
pub use file::{FileCache, ImportHash};
pub use function_call::{FunctionCall, FunctionCallError, FunctionCallResult};
pub use identifiers::{
    CachedFunctionName, CachedModelPath, CachedParameterName, CachedPythonPath, CachedTestIndex,
};
pub use value::{CacheValue, CacheValueConversionError, Interval, Unit};
