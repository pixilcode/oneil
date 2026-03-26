//! Converting Oneil [`Span`] values to LSP types.

use oneil_shared::{
    paths::{ModelPath, PythonPath},
    span::Span,
};
use tower_lsp_server::{
    UriExt,
    lsp_types::{Location, Position, Range, Uri},
};

/// Converts a [`Span`] to an LSP [`Range`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "we know the values are not pointers"
)]
pub const fn span_to_range(span: Span) -> Range {
    Range {
        start: Position {
            line: (span.start().line - 1) as u32,
            character: (span.start().column - 1) as u32,
        },
        end: Position {
            line: (span.end().line - 1) as u32,
            character: (span.end().column - 1) as u32,
        },
    }
}

/// Converts a model path and span to an LSP [`Location`].
pub fn span_to_location(model_path: &ModelPath, span: Span) -> Location {
    let uri = Uri::from_file_path(model_path.as_path()).unwrap_or_else(|| {
        panic!(
            "Failed to convert model path to URI: {}",
            model_path.as_path().display()
        )
    });

    Location {
        uri,
        range: span_to_range(span),
    }
}

/// Converts a Python function line number to an LSP [`Location`].
pub fn python_function_line_to_location(python_path: &PythonPath, line_no: u32) -> Location {
    let uri = Uri::from_file_path(python_path.as_path()).unwrap_or_else(|| {
        panic!(
            "Failed to convert Python path to URI: {}",
            python_path.as_path().display()
        )
    });

    Location {
        uri,
        range: Range {
            start: Position {
                line: line_no - 1,
                character: 0,
            },
            end: Position {
                line: line_no - 1,
                character: 0,
            },
        },
    }
}
