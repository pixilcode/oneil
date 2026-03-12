#![cfg_attr(doc, doc = include_str!("../README.md"))]
//! Intermediate Representation (IR) for the Oneil programming language

mod debug_info;
mod expr;
mod model;
mod model_import;
mod parameter;
mod python_import;
mod test;
mod unit;

pub use debug_info::TraceLevel;
pub use expr::{
    BinaryOp, ComparisonOp, Expr, ExprVisitor, FunctionName, Literal, UnaryOp, Variable,
};
pub use model::Model;
pub use model_import::{ReferenceImport, SubmodelImport};
pub use parameter::{Dependencies, Limits, Parameter, ParameterValue, PiecewiseExpr};
pub use python_import::PythonImport;
pub use test::Test;
pub use unit::{CompositeUnit, DisplayCompositeUnit, DisplayUnit, Unit, UnitInfo};
