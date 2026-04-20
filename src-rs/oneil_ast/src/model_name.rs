//! Model name for the AST

use crate::node::Node;

/// The name of a model in the AST
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelName(String);

/// A node containing a model's name
pub type ModelNameNode = Node<ModelName>;

impl ModelName {
    /// Creates a new model name with the given string value
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns the model name content as a string slice
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns this model name as a string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for ModelName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&String> for ModelName {
    fn from(value: &String) -> Self {
        Self::new(value.clone())
    }
}

impl From<&str> for ModelName {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}
