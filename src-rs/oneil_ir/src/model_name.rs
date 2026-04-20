//! Model names in the intermediate representation.

use oneil_shared::span::Span;

/// The name declared for a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelName {
    value: String,
    span: Span,
}

impl ModelName {
    /// Creates a new model name with the given string value and source span.
    #[must_use]
    pub const fn new(value: String, span: Span) -> Self {
        Self { value, span }
    }

    /// Returns the model name as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.value.as_str()
    }

    /// Returns the source span covering the model name declaration.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns this model name as a string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.value
    }
}
