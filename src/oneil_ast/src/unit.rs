//! Unit expression constructs for the AST

use crate::node::Node;

/// A unit identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitIdentifier(String);

impl UnitIdentifier {
    /// Creates a new unit identifier from a string
    #[must_use]
    pub const fn new(identifier: String) -> Self {
        Self(identifier)
    }

    /// Returns the unit identifier as a string slice
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns this unit identifier as a string
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for UnitIdentifier {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for UnitIdentifier {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

/// A node containing a unit identifier
pub type UnitIdentifierNode = Node<UnitIdentifier>;

/// Represents a unit expression
#[derive(Debug, Clone, PartialEq)]
pub enum UnitExpr {
    /// Binary operation on unit expressions
    BinaryOp {
        /// The unit operator
        op: UnitOpNode,
        /// The left operand
        left: UnitExprNode,
        /// The right operand
        right: UnitExprNode,
    },
    /// Parenthesized unit expression
    Parenthesized {
        /// The expression inside parentheses
        expr: UnitExprNode,
    },
    /// A `1` unit, usually used for units like 1/s
    UnitOne,
    /// A unit with optional exponent
    Unit {
        /// The unit identifier
        identifier: UnitIdentifierNode,
        /// The optional exponent
        exponent: Option<UnitExponentNode>,
    },
}

/// A node containing a unit expression
pub type UnitExprNode = Node<UnitExpr>;

impl UnitExpr {
    /// Creates a binary operation unit expression
    #[must_use]
    pub const fn binary_op(op: UnitOpNode, left: UnitExprNode, right: UnitExprNode) -> Self {
        Self::BinaryOp { op, left, right }
    }

    /// Creates a parenthesized unit expression
    #[must_use]
    pub const fn parenthesized(expr: UnitExprNode) -> Self {
        Self::Parenthesized { expr }
    }

    /// Creates a `1` unit, usually used for units like 1/s
    #[must_use]
    pub const fn unit_one() -> Self {
        Self::UnitOne
    }

    /// Creates a unit expression with optional exponent
    #[must_use]
    pub const fn unit(identifier: UnitIdentifierNode, exponent: Option<UnitExponentNode>) -> Self {
        Self::Unit {
            identifier,
            exponent,
        }
    }
}

/// Unit operators for unit expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitOp {
    /// Multiplication operator for units (*)
    Multiply,
    /// Division operator for units (/)
    Divide,
}

/// A node containing a unit operator
pub type UnitOpNode = Node<UnitOp>;

impl UnitOp {
    /// Creates a multiplication operator for units
    #[must_use]
    pub const fn multiply() -> Self {
        Self::Multiply
    }

    /// Creates a division operator for units
    #[must_use]
    pub const fn divide() -> Self {
        Self::Divide
    }
}

/// A unit exponent value
#[derive(Debug, Clone, PartialEq)]
pub struct UnitExponent(f64);

/// A node containing a unit exponent
pub type UnitExponentNode = Node<UnitExponent>;

impl UnitExponent {
    /// Creates a new unit exponent with the given value
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the value of the unit exponent
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.0
    }
}
