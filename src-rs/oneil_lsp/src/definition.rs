//! Go-to-definition resolution for [`crate::symbol_lookup::SymbolAtPosition`].

use oneil_runtime::Runtime;
use oneil_shared::paths::ModelPath;
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

use crate::{
    location::{python_function_line_to_location, span_to_location},
    symbol_lookup::SymbolAtPosition,
};

/// Resolves a symbol to its definition location.
pub fn resolve_definition(
    symbol: &SymbolAtPosition,
    runtime: &mut Runtime,
    current_model_path: &ModelPath,
) -> Option<Location> {
    match symbol {
        SymbolAtPosition::ParameterDefinition { span, .. } => {
            Some(span_to_location(current_model_path, *span))
        }
        SymbolAtPosition::ParameterReference { name, .. } => {
            let (model, _errors) = runtime.load_ir(current_model_path);
            let model = model?;

            let param = model.get_parameter(name)?;

            Some(span_to_location(current_model_path, param.name_span()))
        }
        SymbolAtPosition::ExternalParameterReference {
            model_path,
            parameter_name,
            ..
        } => {
            let (external_model, _errors) = runtime.load_ir(model_path);
            let external_model = external_model?;

            let param = external_model.get_parameter(parameter_name)?;
            Some(span_to_location(model_path, param.name_span()))
        }
        SymbolAtPosition::ModelImportDefinition { path, .. } => {
            let uri = Uri::from_file_path(path.as_path())?;
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
        SymbolAtPosition::ModelImportReference { reference_name, .. } => {
            let (model, _errors) = runtime.load_ir(current_model_path);
            let model = model?;

            let reference_imports = model.reference_imports();
            let reference = reference_imports.get(reference_name)?;
            Some(span_to_location(current_model_path, *reference.name_span()))
        }
        SymbolAtPosition::PythonImport { path, .. } => {
            let uri = Uri::from_file_path(path.as_path())?;
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
            python_path, name, ..
        } => {
            let function = runtime.lookup_python_function(python_path, name)?;
            let function_line_no = function.get_line_no()?;

            Some(python_function_line_to_location(
                python_path,
                function_line_no,
            ))
        }
        SymbolAtPosition::BuiltinValueReference { .. }
        | SymbolAtPosition::BuiltinFunctionReference { .. } => None,
    }
}
