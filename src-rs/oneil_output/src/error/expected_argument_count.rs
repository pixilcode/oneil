//! Expected arity for validating function calls.

use std::fmt;

/// Represents the expected number of arguments for a function call.
///
/// This enum is used to specify argument count requirements when validating
/// function calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedArgumentCount {
    /// Exactly the specified number of arguments is required.
    Exact(usize),
    /// At least the specified number of arguments is required.
    AtLeast(usize),
    /// At most the specified number of arguments is allowed.
    AtMost(usize),
    /// Between the minimum (inclusive) and maximum (inclusive) number of arguments is required.
    Between(usize, usize),
}

impl fmt::Display for ExpectedArgumentCount {
    /// Formats the expected argument count for diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(1) => write!(f, "1 argument"),
            Self::Exact(count) => write!(f, "{count} arguments"),
            Self::AtLeast(1) => write!(f, "at least 1 argument"),
            Self::AtLeast(count) => write!(f, "at least {count} arguments"),
            Self::AtMost(1) => write!(f, "at most 1 argument"),
            Self::AtMost(count) => write!(f, "at most {count} arguments"),
            Self::Between(min, max) => write!(f, "between {min} and {max} arguments"),
        }
    }
}
