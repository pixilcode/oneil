use crate::{DisplayUnit, ValueType};

/// Errors that can occur when converting a value to a specific unit.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitConversionError {
    /// Unit mismatch between the value unit and the requested target unit.
    UnitMismatch {
        /// Unit of the value being converted.
        value_unit: DisplayUnit,
        /// Unit requested by the caller.
        target_unit: DisplayUnit,
    },
    /// Value type is not convertible to the target unit.
    InvalidType {
        /// Value type of the value that could not be converted.
        value_type: Box<ValueType>,
        /// Requested target unit for the conversion.
        target_unit: Box<DisplayUnit>,
    },
}
