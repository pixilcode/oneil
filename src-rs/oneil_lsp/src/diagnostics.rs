//! Converts runtime evaluation errors into LSP diagnostics.

use indexmap::IndexMap;
use oneil_runtime::output::error::RuntimeErrors;
use oneil_shared::error::{Context, DiagnosticKind, OneilDiagnostic};
use tower_lsp_server::ls_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range, Uri,
};

/// Builds LSP diagnostics for the given URI from runtime errors.
///
/// Only returns diagnostics for the specified path and filters out internal errors
/// that are not useful to show in the editor.
#[must_use]
pub fn diagnostics_from_runtime_errors(errors: &RuntimeErrors) -> IndexMap<Uri, Vec<Diagnostic>> {
    errors
        .to_map()
        .into_iter()
        .filter_map(|(path, errors)| {
            let uri = Uri::from_file_path(path)?;
            Some((
                uri,
                errors
                    .iter()
                    .filter(|error| !error.is_internal_error())
                    .map(oneil_diagnostic_to_lsp)
                    .collect(),
            ))
        })
        .collect()
}

/// Converts a single [`OneilDiagnostic`] to an LSP [`Diagnostic`], if it has a valid location.
fn oneil_diagnostic_to_lsp(error: &OneilDiagnostic) -> Diagnostic {
    let range = error.location().map_or_else(
        || Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        error_location_to_range,
    );

    let severity = error_kind_to_severity(error.kind());
    let message = build_diagnostic_message(error);
    let related_information = build_related_information(error);

    Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        source: Some("oneil".to_string()),
        message,
        related_information,
        tags: None,
        code_description: None,
        data: None,
    }
}

const fn error_kind_to_severity(kind: DiagnosticKind) -> DiagnosticSeverity {
    match kind {
        DiagnosticKind::Error => DiagnosticSeverity::ERROR,
    }
}

/// Builds the diagnostic message from the error and its context.
fn build_diagnostic_message(error: &OneilDiagnostic) -> String {
    let base = error.message().to_string();

    let context_lines: Vec<String> = error.context().iter().map(context_to_string).collect();

    if context_lines.is_empty() {
        base
    } else {
        format!("{}\n\n{}", base, context_lines.join("\n"))
    }
}

/// Formats a [`Context`] as a string for display.
fn context_to_string(ctx: &Context) -> String {
    match ctx {
        Context::Note(s) => format!("note: {s}"),
        Context::Help(s) => format!("help: {s}"),
    }
}

/// Builds LSP related information from context-with-source entries.
fn build_related_information(error: &OneilDiagnostic) -> Option<Vec<DiagnosticRelatedInformation>> {
    let path = error.path();
    let uri = Uri::from_file_path(path)?;

    let related: Vec<DiagnosticRelatedInformation> = error
        .context_with_source()
        .iter()
        .map(|(ctx, loc)| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: error_location_to_range(loc),
            },
            message: context_to_string(ctx),
        })
        .collect();

    if related.is_empty() {
        None
    } else {
        Some(related)
    }
}

/// Converts an [`oneil_shared::error::ErrorLocation`] to an LSP [`Range`].
///
/// `ErrorLocation` uses 1-indexed line and column; LSP uses 0-indexed.
#[expect(
    clippy::cast_possible_truncation,
    reason = "line and column values are from source and fit in u32"
)]
fn error_location_to_range(loc: &oneil_shared::error::ErrorLocation) -> Range {
    let line = (loc.line().saturating_sub(1)) as u32;
    let col = (loc.column().saturating_sub(1)) as u32;
    let length = loc.length() as u32;

    Range {
        start: Position {
            line,
            character: col,
        },
        end: Position {
            line,
            // NOTE: this assumes that the location starts and ends on the same line
            character: col.saturating_add(length),
        },
    }
}
