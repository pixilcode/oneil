use std::fmt;

use crate::{DisplayUnit, NumberType, ValueType};

/// Represents the expected type for type checking operations in value-level errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedType {
    /// A boolean value.
    Boolean,
    /// A string value.
    String,
    /// A unitless number (scalar or interval without units).
    Number {
        /// The type of the expected number, if specified.
        number_type: Option<NumberType>,
    },
    /// A number with a unit (measured number).
    MeasuredNumber {
        /// The type of the expected number, if specified.
        number_type: Option<NumberType>,
        /// The unit of the expected measured number, if specified.
        unit: Option<DisplayUnit>,
    },
    /// Either a unitless number or a number with a unit.
    NumberOrMeasuredNumber {
        /// The type of the expected number, if specified.
        number_type: Option<NumberType>,
    },
}

impl ExpectedType {
    /// Returns an expected type with the same kind as `value_type` (boolean, string, unitless
    /// number, or measured number including display unit), while treating scalar vs interval as
    /// unspecified (`number_type: None` on number variants).
    ///
    /// ```
    /// # use oneil_output::{ExpectedType, NumberType, ValueType};
    /// let vt = ValueType::Number {
    ///     number_type: NumberType::Interval,
    /// };
    /// assert_eq!(
    ///     ExpectedType::matching_value_type_ignoring_number_kind(&vt),
    ///     ExpectedType::Number { number_type: None }
    /// );
    /// ```
    #[must_use]
    pub fn matching_value_type_ignoring_number_kind(value_type: &ValueType) -> Self {
        match value_type {
            ValueType::Boolean => Self::Boolean,
            ValueType::String => Self::String,
            ValueType::Number { .. } => Self::Number { number_type: None },
            ValueType::MeasuredNumber { unit, .. } => Self::MeasuredNumber {
                number_type: None,
                unit: Some(unit.display_unit.clone()),
            },
        }
    }
}

impl fmt::Display for ExpectedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let number_type_to_string = |type_: &Option<NumberType>| match type_ {
            Some(NumberType::Scalar) => "scalar",
            Some(NumberType::Interval) => "interval",
            None => "number",
        };

        let unit_to_string = |unit: &Option<DisplayUnit>| {
            unit.as_ref()
                .map_or_else(|| "a unit".to_string(), |unit| format!("unit `{unit}`"))
        };

        match self {
            Self::Boolean => write!(f, "boolean"),
            Self::String => write!(f, "string"),
            Self::Number { number_type: type_ } => {
                let type_str = number_type_to_string(type_);
                write!(f, "unitless {type_str}")
            }
            Self::MeasuredNumber {
                number_type: type_,
                unit,
            } => {
                let type_str = number_type_to_string(type_);
                let unit_str = unit_to_string(unit);
                write!(f, "{type_str} with {unit_str}")
            }
            Self::NumberOrMeasuredNumber { number_type: type_ } => {
                let type_str = number_type_to_string(type_);
                write!(f, "{type_str}")
            }
        }
    }
}
