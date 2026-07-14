//! The [`EvalWarning`] enum and its trait implementations.

use std::fmt;

use oneil_shared::{
    error::{AsOneilDiagnostic, Context as ErrorContext, DiagnosticKind, ErrorLocation},
    span::Span,
    symbols::PyFunctionName,
};

/// Non-fatal issues produced while evaluating expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalWarning {
    /// Python function evaluation failed and a fallback result was used instead.
    UsedFallback {
        /// The name of the Python function that was called.
        function_name: PyFunctionName,
        /// The source span of the function call.
        function_call_span: Span,
        /// The error message from Python or from conversion.
        message: String,
        /// The traceback from Python.
        traceback: Option<String>,
    },
}

impl fmt::Display for EvalWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UsedFallback {
                function_name,
                function_call_span: _,
                message: _,
                traceback: _,
            } => {
                let function_name = function_name.as_str();
                write!(
                    f,
                    "python function `{function_name}` failed; using fallback"
                )
            }
        }
    }
}

impl AsOneilDiagnostic for EvalWarning {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Warning
    }

    fn message(&self) -> String {
        self.to_string()
    }

    fn diagnostic_location(&self, source: &str) -> Option<ErrorLocation> {
        match self {
            Self::UsedFallback {
                function_name: _,
                function_call_span,
                message: _,
                traceback: _,
            } => Some(ErrorLocation::from_source_and_span(
                source,
                *function_call_span,
            )),
        }
    }

    fn context(&self) -> Vec<ErrorContext> {
        match self {
            Self::UsedFallback {
                function_name: _,
                function_call_span: _,
                message,
                traceback,
            } => traceback.as_ref().map_or_else(
                || vec![ErrorContext::Note(message.clone())],
                |traceback| {
                    vec![
                        ErrorContext::Note(message.clone()),
                        ErrorContext::Note(traceback.clone()),
                    ]
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EvalWarning;
    use oneil_shared::error::AsOneilDiagnostic;
    use oneil_shared::span::{SourceLocation, Span};
    use oneil_shared::symbols::PyFunctionName;

    fn tiny_span() -> Span {
        Span::new(
            SourceLocation {
                offset: 0,
                line: 1,
                column: 1,
            },
            SourceLocation {
                offset: 1,
                line: 1,
                column: 2,
            },
        )
    }

    #[test]
    fn used_fallback_display() {
        let warning = EvalWarning::UsedFallback {
            function_name: PyFunctionName::from("my_fn"),
            function_call_span: tiny_span(),
            message: "x".into(),
            traceback: None,
        };

        assert_eq!(
            warning.to_string(),
            "python function `my_fn` failed; using fallback"
        );
    }

    #[test]
    fn used_fallback_diagnostic_kind_and_context() {
        let warning = EvalWarning::UsedFallback {
            function_name: PyFunctionName::from("f"),
            function_call_span: tiny_span(),
            message: "err".into(),
            traceback: Some("tb".into()),
        };

        assert_eq!(warning.kind(), oneil_shared::error::DiagnosticKind::Warning);
        assert_eq!(warning.context().len(), 2);
    }
}
