//! Builtin documentation and lookup for the runtime.

use oneil_output::Value;
use oneil_shared::symbols::{BuiltinFunctionName, BuiltinValueName, UnitBaseName, UnitPrefix};

use super::Runtime;

impl Runtime {
    /// Returns documentation for all builtin units.
    pub fn builtin_units_docs(&self) -> impl Iterator<Item = (&'static str, Vec<&UnitBaseName>)> {
        self.builtins.builtin_units_docs()
    }

    /// Returns documentation for all builtin functions.
    pub fn builtin_functions_docs(
        &self,
    ) -> impl Iterator<
        Item = (
            &BuiltinFunctionName,
            (&'static [&'static str], &'static str),
        ),
    > {
        self.builtins.builtin_functions_docs()
    }

    /// Returns documentation for all builtin values.
    pub fn builtin_values_docs(
        &self,
    ) -> impl Iterator<Item = (&BuiltinValueName, (&'static str, Value))> {
        self.builtins.builtin_values_docs()
    }

    /// Returns documentation for all builtin prefixes.
    pub fn builtin_prefixes_docs(
        &self,
    ) -> impl Iterator<Item = (&UnitPrefix, (&'static str, f64))> {
        self.builtins.builtin_prefixes_docs()
    }
}
