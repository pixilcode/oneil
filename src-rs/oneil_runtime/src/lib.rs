#![cfg_attr(doc, doc = include_str!("../README.md"))]
//! Runtime for the Oneil programming language

mod cache;
mod error;
mod runtime;

pub mod output;

#[cfg(feature = "python")]
pub use cache::{PythonCacheReadStrategy, PythonCacheStrategy};
pub use runtime::Runtime;
