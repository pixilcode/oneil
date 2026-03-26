//! Number types: scalar/interval, normalized, and measured.

mod measured_number;
mod normalized_number;
mod number;

pub use measured_number::MeasuredNumber;
pub use normalized_number::NormalizedNumber;
pub use number::Number;
