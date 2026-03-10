//! Symbol lookup utilities for finding definitions in Oneil models

use oneil_runtime::Runtime;
use oneil_runtime::output::ir;
use oneil_shared::span::Span;
use tower_lsp_server::UriExt;
use tower_lsp_server::lsp_types::{Location, Position, Range, Uri};

/// Represents a symbol found at a cursor position
#[derive(Debug, Clone)]
pub enum SymbolAtPosition {
    /// A parameter definition (cursor is on the parameter name in its declaration)
    ParameterDefinition { span: Span },
    /// A parameter reference (cursor is on a parameter used in an expression)
    ParameterReference { name: ir::ParameterName },
    /// An external parameter reference (e.g., `x.model_name`)
    ///
    /// This occurs when the cursor is on the parameter name part (e.g., `x`)
    ExternalParameterReference {
        model_path: ir::ModelPath,
        parameter_name: ir::ParameterName,
    },
    /// A submodel or reference import name
    ModelImportDefinition { name: String, path: ir::ModelPath },
    /// A reference to a model import (e.g., `x.model_name`)
    ///
    /// This occurs when the cursor is on the model name part (e.g., `model_name`)
    ModelImportReference { reference_name: ir::ReferenceName },
    /// A python import (e.g., `import math`)
    PythonImport { path: ir::PythonPath },
    /// A python function reference
    PythonFunctionReference {
        python_path: ir::PythonPath,
        name: ir::Identifier,
    },
    /// A builtin function reference
    BuiltinFunctionReference { name: ir::Identifier },
}

/// Finds the symbol at a given byte offset in a model
pub fn find_symbol_at_offset(
    model: oneil_runtime::output::reference::ModelIrReference<'_>,
    offset: usize,
) -> Option<SymbolAtPosition> {
    // Check if cursor is on a parameter definition or in the parameter expressions
    for param in model.parameters().values() {
        // Check if cursor is on the parameter name
        if span_contains_offset(param.name_span(), offset) {
            return Some(SymbolAtPosition::ParameterDefinition {
                span: param.name_span(),
            });
        }

        // Check if cursor is on the parameter value
        if let Some(symbol) = find_symbol_in_parameter_value(param.value(), offset) {
            return Some(symbol);
        }

        // Check if the cursor is on the parameter limits
        if let Some(value) = find_symbol_in_limits(param.limits(), offset) {
            return Some(value);
        }
    }

    // Check if cursor is on a submodel import name
    for (submodel_name, submodel_import) in model.submodel_models() {
        if span_contains_offset(*submodel_import.name_span(), offset) {
            let submodel_path = submodel_import.reference_import().path().clone();

            return Some(SymbolAtPosition::ModelImportDefinition {
                name: submodel_name.to_string(),
                path: submodel_path,
            });
        }
    }

    // Check if cursor is on a reference import name
    for (reference_name, reference_import) in model.reference_models() {
        if span_contains_offset(*reference_import.name_span(), offset) {
            return Some(SymbolAtPosition::ModelImportDefinition {
                name: reference_name.to_string(),
                path: reference_import.path().clone(),
            });
        }
    }

    // Check if cursor is on a python import
    for (python_path, python_import) in model.python_imports() {
        if span_contains_offset(*python_import.import_path_span(), offset) {
            return Some(SymbolAtPosition::PythonImport {
                path: python_path.clone(),
            });
        }
    }

    None
}

fn find_symbol_in_limits(limits: &ir::Limits, offset: usize) -> Option<SymbolAtPosition> {
    match limits {
        ir::Limits::Default => {}
        ir::Limits::Continuous {
            min,
            max,
            limit_expr_span: _,
        } => {
            if let Some(symbol) = find_symbol_in_expr(min, offset) {
                return Some(symbol);
            }
            if let Some(symbol) = find_symbol_in_expr(max, offset) {
                return Some(symbol);
            }
        }
        ir::Limits::Discrete {
            values,
            limit_expr_span: _,
        } => {
            for value in values {
                if let Some(symbol) = find_symbol_in_expr(value, offset) {
                    return Some(symbol);
                }
            }
        }
    }
    None
}

/// Finds a symbol in a parameter value expression
fn find_symbol_in_parameter_value(
    value: &ir::ParameterValue,
    offset: usize,
) -> Option<SymbolAtPosition> {
    match value {
        ir::ParameterValue::Simple(expr, _) => find_symbol_in_expr(expr, offset),
        ir::ParameterValue::Piecewise(exprs, _) => {
            for piecewise_expr in exprs {
                if let Some(symbol) = find_symbol_in_expr(piecewise_expr.expr(), offset) {
                    return Some(symbol);
                }
                if let Some(symbol) = find_symbol_in_expr(piecewise_expr.if_expr(), offset) {
                    return Some(symbol);
                }
            }
            None
        }
    }
}

/// Recursively finds a symbol in an expression
fn find_symbol_in_expr(expr: &ir::Expr, offset: usize) -> Option<SymbolAtPosition> {
    match expr {
        ir::Expr::Variable { span, variable } => {
            if !span_contains_offset(*span, offset) {
                return None;
            }

            match variable {
                ir::Variable::Parameter {
                    parameter_name,
                    parameter_span,
                } => span_contains_offset(*parameter_span, offset).then(|| {
                    SymbolAtPosition::ParameterReference {
                        name: parameter_name.clone(),
                    }
                }),
                ir::Variable::External {
                    model_path,
                    reference_name,
                    reference_span,
                    parameter_name,
                    parameter_span,
                } => {
                    // Check if cursor is on the model name or parameter name
                    if span_contains_offset(*reference_span, offset) {
                        // Cursor is on the model name part
                        Some(SymbolAtPosition::ModelImportReference {
                            reference_name: reference_name.clone(),
                        })
                    } else if span_contains_offset(*parameter_span, offset) {
                        // Cursor is on the parameter name part
                        Some(SymbolAtPosition::ExternalParameterReference {
                            model_path: model_path.clone(),
                            parameter_name: parameter_name.clone(),
                        })
                    } else {
                        None
                    }
                }
                ir::Variable::Builtin { .. } => None, // Builtins don't have definitions
            }
        }
        ir::Expr::ComparisonOp {
            left,
            right,
            rest_chained,
            ..
        } => {
            if let Some(symbol) = find_symbol_in_expr(left, offset) {
                return Some(symbol);
            }
            if let Some(symbol) = find_symbol_in_expr(right, offset) {
                return Some(symbol);
            }
            for (_, chained_expr) in rest_chained {
                if let Some(symbol) = find_symbol_in_expr(chained_expr, offset) {
                    return Some(symbol);
                }
            }
            None
        }
        ir::Expr::BinaryOp { left, right, .. } => {
            if let Some(symbol) = find_symbol_in_expr(left, offset) {
                return Some(symbol);
            }
            find_symbol_in_expr(right, offset)
        }
        ir::Expr::UnaryOp { expr, .. } | ir::Expr::UnitCast { expr, .. } => {
            find_symbol_in_expr(expr, offset)
        }
        ir::Expr::FunctionCall {
            span: _,
            name_span,
            name,
            args,
        } => {
            // Check if cursor is on the function name
            if span_contains_offset(*name_span, offset) {
                match name {
                    ir::FunctionName::Builtin(name, _name_span) => {
                        return Some(SymbolAtPosition::BuiltinFunctionReference {
                            name: name.clone(),
                        });
                    }
                    ir::FunctionName::Imported {
                        python_path,
                        name,
                        name_span: _,
                    } => {
                        return Some(SymbolAtPosition::PythonFunctionReference {
                            python_path: python_path.clone(),
                            name: name.clone(),
                        });
                    }
                }
            }

            // Check if cursor is on an argument
            for arg in args {
                if let Some(symbol) = find_symbol_in_expr(arg, offset) {
                    return Some(symbol);
                }
            }
            None
        }
        ir::Expr::Literal { .. } => None,
    }
}

/// Resolves a symbol to its definition location
pub fn resolve_definition(
    symbol: &SymbolAtPosition,
    runtime: &mut Runtime,
    current_model_path: &ir::ModelPath,
) -> Option<Location> {
    match symbol {
        SymbolAtPosition::ParameterDefinition { span } => {
            // Already at the definition
            Some(span_to_location(current_model_path, *span))
        }
        SymbolAtPosition::ParameterReference { name } => {
            // Find the parameter in the current model
            let (model, _errors) = runtime.load_ir(current_model_path);
            let model = model?;

            let param = model.get_parameter(name)?;

            Some(span_to_location(current_model_path, param.name_span()))
        }
        SymbolAtPosition::ExternalParameterReference {
            model_path,
            parameter_name,
        } => {
            // Find the parameter in the external model
            let (external_model, _errors) = runtime.load_ir(model_path);
            let external_model = external_model?;

            let param = external_model.get_parameter(parameter_name)?;
            Some(span_to_location(model_path, param.name_span()))
        }
        SymbolAtPosition::ModelImportDefinition { path, .. } => {
            // Navigate to the imported model file
            let uri = Uri::from_file_path(path.as_ref())?;
            Some(Location {
                uri,
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
            })
        }
        SymbolAtPosition::ModelImportReference { reference_name } => {
            // Find the reference in the current model
            let (model, _errors) = runtime.load_ir(current_model_path);
            let model = model?;

            let reference_imports = model.reference_imports();
            let reference = reference_imports.get(reference_name)?;
            Some(span_to_location(current_model_path, *reference.name_span()))
        }
        SymbolAtPosition::PythonImport { path } => {
            // Navigate to the python import file
            let uri = Uri::from_file_path(path.as_ref())?;
            Some(Location {
                uri,
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
            })
        }
        SymbolAtPosition::PythonFunctionReference {
            python_path,
            name: _,
        } => {
            // For now, we don't have a way to send them to the exact location
            // of the function definition, so we just send them to the python import
            let (model, _errors) = runtime.load_ir(current_model_path);
            let model = model?;

            let python_imports = model.python_imports();
            let python_import = python_imports.get(python_path)?;
            Some(span_to_location(
                current_model_path,
                *python_import.import_path_span(),
            ))
        }
        SymbolAtPosition::BuiltinFunctionReference { .. } => {
            // For now at least, we don't have builtin function definitions
            None
        }
    }
}

/// Checks if a span contains a given byte offset
const fn span_contains_offset(span: Span, offset: usize) -> bool {
    span.start().offset <= offset && offset < span.end().offset
}

/// Converts a Span to an LSP Location
fn span_to_location(model_path: &ir::ModelPath, span: Span) -> Location {
    let uri = Uri::from_file_path(model_path.as_ref()).unwrap_or_else(|| {
        panic!(
            "Failed to convert model path to URI: {}",
            model_path.as_ref().display()
        )
    });
    Location {
        uri,
        range: span_to_range(span),
    }
}

/// Converts a Span to an LSP Range
#[expect(
    clippy::cast_possible_truncation,
    reason = "we know the values are not pointers"
)]
const fn span_to_range(span: Span) -> Range {
    Range {
        start: Position {
            line: (span.start().line - 1) as u32, // Span uses 1-indexed lines, LSP uses 0-indexed
            character: (span.start().column - 1) as u32, // Span uses 1-indexed columns, LSP uses 0-indexed
        },
        end: Position {
            line: (span.end().line - 1) as u32,
            character: (span.end().column - 1) as u32,
        },
    }
}
