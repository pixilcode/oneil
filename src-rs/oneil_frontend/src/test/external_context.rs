//! Test implementation of [`ExternalResolutionContext`].
//!
//! Provides a configurable external context for tests that need builtin
//! lookups, model AST loading, and/or Python import validation.
//! [`load_ast`](ExternalResolutionContext::load_ast) returns ASTs registered via
//! [`with_model_asts`](TestExternalContext::with_model_asts); otherwise returns `Err(())`.
//! [`load_python_import`](ExternalResolutionContext::load_python_import) returns
//! `Ok(&IndexSet<PyFunctionName>)` (the set of function names) for paths registered via
//! [`with_python_imports_ok`](TestExternalContext::with_python_imports_ok); use
//! [`with_python_import_functions`](TestExternalContext::with_python_import_functions) to set the function list.

use indexmap::{IndexMap, IndexSet};
use oneil_ast as ast;
use oneil_output::Unit;
use oneil_shared::{
    load_result::LoadResult,
    paths::{ModelPath, PythonPath},
    symbols::{BuiltinFunctionName, BuiltinValueName, PyFunctionName, UnitBaseName, UnitPrefix},
};

use crate::{
    AstLoadingFailedError, ExternalResolutionContext, PythonImportLoadingFailedError,
    ResolutionErrorCollection, instance::InstancedModel,
};

pub struct TestBuiltinUnit {
    pub name: &'static str,
    pub supports_si_prefixes: bool,
}

/// Test double for [`ExternalResolutionContext`].
///
/// Configurable builtin values, builtin functions, model ASTs (via
/// [`with_model_asts`](Self::with_model_asts)), and Python import paths with
/// their function lists (via [`with_python_imports_ok`](Self::with_python_imports_ok) and
/// [`with_python_import_functions`](Self::with_python_import_functions)).
#[derive(Debug, Default)]
pub struct TestExternalContext {
    /// Builtin variables that are valid.
    builtin_variables: IndexSet<BuiltinValueName>,

    /// Builtin functions that are valid.
    builtin_functions: IndexSet<BuiltinFunctionName>,

    /// Builtin units that are valid.
    builtin_units: IndexSet<UnitBaseName>,

    /// Resolved [`Unit`]s for builtin base names.
    ///
    /// Tests that exercise unit *resolution* (where the lowering pass needs
    /// the dimension map of each base unit) populate this map. Tests that
    /// only need to know whether a name *is* a builtin can leave it empty;
    /// `lookup_unit` will return `None` and the resulting [`ir::CompositeUnit`]
    /// will fall back to a dimensionless dimension map.
    builtin_unit_definitions: IndexMap<UnitBaseName, Unit>,

    /// Units that support SI prefixes.
    units_with_si_prefixes: IndexSet<UnitBaseName>,

    /// Builtin prefixes (name -> magnitude).
    builtin_prefixes: IndexMap<UnitPrefix, f64>,

    /// Model path -> AST; paths are derived from the given path's stem (e.g. "test.on" -> ModelPath("test.on")).
    model_asts: IndexMap<ModelPath, ast::ModelNode>,

    /// Python path (with `.py` extension) -> set of callable function names returned by `load_python_import`.
    python_imports: IndexMap<PythonPath, IndexSet<PyFunctionName>>,
}

impl TestExternalContext {
    /// Creates a new empty test external context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the given identifiers as builtin values.
    #[must_use]
    pub fn with_builtin_variables(
        mut self,
        variables: impl IntoIterator<Item = &'static str>,
    ) -> Self {
        self.builtin_variables
            .extend(variables.into_iter().map(BuiltinValueName::from));

        self
    }

    /// Registers the given identifiers as builtin functions.
    #[must_use]
    pub fn with_builtin_functions(
        mut self,
        functions: impl IntoIterator<Item = &'static str>,
    ) -> Self {
        self.builtin_functions
            .extend(functions.into_iter().map(BuiltinFunctionName::from));

        self
    }

    /// Registers the given names as builtin units.
    #[must_use]
    pub fn with_builtin_units(mut self, units: impl IntoIterator<Item = TestBuiltinUnit>) -> Self {
        let (builtin_units, units_with_si_prefixes): (
            Vec<UnitBaseName>,
            Vec<Option<UnitBaseName>>,
        ) = units
            .into_iter()
            .map(|unit| {
                (
                    UnitBaseName::from(unit.name),
                    unit.supports_si_prefixes
                        .then_some(UnitBaseName::from(unit.name)),
                )
            })
            .unzip();

        self.builtin_units.extend(builtin_units);

        self.units_with_si_prefixes
            .extend(units_with_si_prefixes.into_iter().flatten());

        self
    }

    /// Registers a resolved [`Unit`] for a builtin base name.
    ///
    /// Use this when the lowering pass should be able to compute a real
    /// dimension map for the unit (e.g. when constructing IR that will be
    /// fed through the apply pass). Plain unit *recognition* tests don't
    /// need to call this — leave it empty and the resulting composite unit
    /// will have a dimensionless dimension map.
    #[must_use]
    #[expect(
        dead_code,
        reason = "kept on the test-context API for tests yet to be added that exercise lowering with realistic units"
    )]
    pub fn with_resolved_unit(mut self, name: &'static str, unit: Unit) -> Self {
        self.builtin_unit_definitions
            .insert(UnitBaseName::from(name), unit);
        self
    }

    /// Registers the given prefixes with their magnitudes.
    #[must_use]
    pub fn with_builtin_prefixes(
        mut self,
        prefixes: impl IntoIterator<Item = (&'static str, f64)>,
    ) -> Self {
        self.builtin_prefixes
            .extend(prefixes.into_iter().map(|(k, v)| (UnitPrefix::from(k), v)));

        self
    }

    /// Registers model ASTs for [`load_ast`](ExternalResolutionContext::load_ast).
    #[must_use]
    pub fn with_model_asts(
        mut self,
        models: impl IntoIterator<Item = (impl AsRef<std::path::Path>, ast::ModelNode)>,
    ) -> Self {
        for (path, model) in models {
            self.model_asts
                .insert(ModelPath::from_path_with_ext(path.as_ref()), model);
        }
        self
    }

    /// Registers Python paths for which `load_python_import` should return `Ok` with an empty function set.
    ///
    /// Paths are compared against the resolved Python path (as from
    /// `model_path.get_sibling_path(import_path)` with `.py` extension). Use the
    /// same path strings as in the import (e.g. `"my_python"`, `"subdir/my_python"`).
    /// Use [`with_python_import_functions`](Self::with_python_import_functions) to set the function list for a path.
    #[must_use]
    pub fn with_python_imports_ok(
        mut self,
        paths: impl IntoIterator<Item = impl AsRef<std::path::Path>>,
    ) -> Self {
        for p in paths {
            let path_str = p.as_ref().to_string_lossy().to_string();

            self.python_imports
                .insert(PythonPath::from_str_no_ext(&path_str), IndexSet::new());
        }
        self
    }

    /// Registers the set of function names for a Python import path.
    ///
    /// The path is normalized with a `.py` extension to match how the resolver looks it up.
    /// If the path was not already registered (e.g. via [`with_python_imports_ok`](Self::with_python_imports_ok)),
    /// it is added.
    #[must_use]
    pub fn with_python_import_functions(
        mut self,
        path: impl AsRef<std::path::Path>,
        functions: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let python_path = PythonPath::from_str_no_ext(&path_str);

        let set: IndexSet<PyFunctionName> = functions
            .into_iter()
            .map(|s| PyFunctionName::from(s.as_ref()))
            .collect();

        self.python_imports.insert(python_path, set);

        self
    }
}

impl ExternalResolutionContext for TestExternalContext {
    fn has_builtin_value(&self, identifier: &ast::Identifier) -> bool {
        self.builtin_variables.contains(identifier)
    }

    fn has_builtin_function(&self, identifier: &ast::Identifier) -> bool {
        self.builtin_functions.contains(identifier)
    }

    fn get_builtin_value_names(&self) -> impl Iterator<Item = &BuiltinValueName> {
        self.builtin_variables.iter()
    }

    fn get_builtin_function_names(&self) -> impl Iterator<Item = &BuiltinFunctionName> {
        self.builtin_functions.iter()
    }

    fn has_builtin_unit(&self, name: &str) -> bool {
        let name = UnitBaseName::from(name);
        self.builtin_units.contains(&name)
    }

    fn available_prefixes(&self) -> impl Iterator<Item = (&UnitPrefix, f64)> {
        self.builtin_prefixes.iter().map(|(k, v)| (k, *v))
    }

    fn unit_supports_si_prefixes(&self, name: &UnitBaseName) -> bool {
        self.units_with_si_prefixes.contains(name)
    }

    fn lookup_unit(&self, name: &UnitBaseName) -> Option<&Unit> {
        self.builtin_unit_definitions.get(name)
    }

    fn load_ast(&mut self, path: &ModelPath) -> LoadResult<&ast::ModelNode, AstLoadingFailedError> {
        self.model_asts
            .get(path)
            .map_or_else(LoadResult::failure, LoadResult::success)
    }

    fn load_python_import<'context>(
        &'context mut self,
        python_path: &PythonPath,
    ) -> Result<IndexSet<&'context PyFunctionName>, PythonImportLoadingFailedError> {
        self.python_imports
            .get(python_path)
            .map(|set| set.iter().collect())
            .ok_or(PythonImportLoadingFailedError)
    }

    #[expect(unreachable_code, reason = "this is unused in tests")]
    fn get_preloaded_models(
        &self,
    ) -> impl Iterator<Item = (ModelPath, InstancedModel, ResolutionErrorCollection)> {
        (unimplemented!("this is unused in tests") as Vec<_>).into_iter()
    }
}
