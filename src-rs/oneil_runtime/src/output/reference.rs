//! References into evaluation results and error contexts.
//!
//! Provides [`ModelReference`] for navigating evaluated models,
//! [`EvalErrorReference`] for inspecting evaluation failures,
//! [`ModelIrReference`] for navigating resolved IR models, and
//! [`ResolutionErrorReference`] for inspecting resolution failures.

use crate::output;
use indexmap::IndexMap;
use oneil_ir as ir;
use oneil_shared::{
    paths::{ModelPath, PythonPath},
    span::Span,
    symbols::{ParameterName, ReferenceName, SubmodelName, TestIndex},
};

use crate::cache::{EvalCache, IrCache};

/// A reference to an evaluated model within a model hierarchy.
///
/// This stores a reference to a model and a reference to the
/// entire model collection.
#[derive(Debug, Clone, Copy)]
pub struct ModelReference<'runtime> {
    model: &'runtime output::Model,
    eval_cache: &'runtime EvalCache,
}

impl<'runtime> ModelReference<'runtime> {
    /// Creates a new `ModelReference` for the given model and evaluation cache.
    #[must_use]
    pub const fn new(model: &'runtime output::Model, eval_cache: &'runtime EvalCache) -> Self {
        Self { model, eval_cache }
    }

    /// Returns the file path of this model.
    #[must_use]
    pub const fn path(&self) -> &'runtime ModelPath {
        &self.model.path
    }

    /// Returns a map of submodel names to their model references or evaluation errors.
    ///
    /// # Panics
    ///
    /// Panics if any submodel has not been visited and
    /// added to the model collection. This should never be
    /// the case as long as creating the `EvalResult`
    /// resolves successfully.
    #[must_use]
    pub fn submodels(&self) -> IndexMap<&'runtime SubmodelName, &'runtime ReferenceName> {
        self.model.submodels.iter().collect()
    }

    /// Returns a map of reference names to their model references or evaluation errors.
    ///
    /// # Panics
    ///
    /// Panics if any reference has not been visited and
    /// added to the model collection. This should never be
    /// the case as long as creating the `EvalResult`
    /// resolves successfully.
    #[must_use]
    pub fn references(&self) -> IndexMap<&'runtime ReferenceName, Self> {
        self.model
            .references
            .iter()
            .filter_map(|(reference_name, model_path)| {
                let entry = self
                    .eval_cache
                    .get_entry(model_path)
                    .expect("reference should be in cache");

                let model = entry.value()?;

                let result = Self {
                    model,
                    eval_cache: self.eval_cache,
                };

                Some((reference_name, result))
            })
            .collect()
    }

    /// Returns a map of parameter names to their evaluated parameter data.
    #[must_use]
    pub fn parameters(&self) -> IndexMap<&'runtime str, &'runtime output::Parameter> {
        self.model
            .parameters
            .iter()
            .map(|(name, parameter)| (name.as_str(), parameter))
            .collect()
    }

    /// Returns the list of evaluated test results for this model.
    #[must_use]
    pub const fn tests(&self) -> &'runtime IndexMap<TestIndex, output::Test> {
        &self.model.tests
    }

    /// Returns the list of model paths that were successfully evaluated.
    #[must_use]
    pub fn all_model_paths(&self) -> Vec<ModelPath> {
        let mut paths = Vec::new();
        self.all_model_paths_internal(&mut paths);
        paths
    }

    fn all_model_paths_internal(&self, paths: &mut Vec<ModelPath>) {
        paths.push(self.model.path.clone());

        for reference_model in self.references().values() {
            reference_model.all_model_paths_internal(paths);
        }
    }
}

/// A reference to a resolved IR model within a model hierarchy.
///
/// This stores a reference to an IR model, the path it was loaded from,
/// and a reference to the IR cache.
#[derive(Debug, Clone, Copy)]
pub struct ModelIrReference<'runtime> {
    model: &'runtime ir::Model,
    ir_cache: &'runtime IrCache,
}

impl<'runtime> ModelIrReference<'runtime> {
    /// Creates a new `ModelIrReference` for the given model, IR cache, and path.
    #[must_use]
    pub const fn new(model: &'runtime ir::Model, ir_cache: &'runtime IrCache) -> Self {
        Self { model, ir_cache }
    }

    /// Returns the path of this model.
    #[must_use]
    pub const fn path(&self) -> &'runtime ModelPath {
        self.model.get_path()
    }

    /// Returns the declared model name, if one was provided in source.
    #[must_use]
    pub const fn name(&self) -> Option<&'runtime ir::ModelName> {
        self.model.name()
    }

    /// Returns the optional model-level documentation note.
    #[must_use]
    pub const fn note(&self) -> Option<&'runtime ir::Note> {
        self.model.note()
    }

    /// Returns a map of submodel names to their `SubmodelImport`s.
    ///
    /// If you need the model reference itself, use `submodel_models` instead.
    #[must_use]
    pub fn submodel_imports(
        &self,
    ) -> IndexMap<&'runtime SubmodelName, &'runtime ir::SubmodelImport> {
        self.model.get_submodels().iter().collect()
    }

    /// Returns a map of submodel names to their IR model references.
    ///
    /// # Panics
    ///
    /// Panics if any submodel's reference has not been visited and
    /// added to the IR cache.
    #[must_use]
    pub fn submodel_models(
        &self,
    ) -> IndexMap<&'runtime SubmodelName, SubmodelImportReference<'runtime>> {
        self.model
            .get_submodels()
            .iter()
            .map(|(name, submodel_import)| {
                (
                    name,
                    SubmodelImportReference::new(
                        submodel_import,
                        self.ir_cache,
                        self.model.get_references(),
                    ),
                )
            })
            .collect()
    }

    /// Returns a map of reference names to their `ReferenceImport`s.
    ///
    /// If you need the model reference itself, use `reference_models` instead.
    #[must_use]
    pub fn reference_imports(
        &self,
    ) -> IndexMap<&'runtime ReferenceName, &'runtime ir::ReferenceImport> {
        self.model.get_references().iter().collect()
    }

    /// Returns a map of reference names to their IR model references
    ///
    /// # Panics
    ///
    /// Panics if any reference has not been visited and
    /// added to the IR cache.
    #[must_use]
    pub fn reference_models(
        &self,
    ) -> IndexMap<&'runtime ReferenceName, ReferenceImportReference<'runtime>> {
        self.model
            .get_references()
            .iter()
            .map(|(name, reference_import)| {
                (
                    name,
                    ReferenceImportReference::new(reference_import, self.ir_cache),
                )
            })
            .collect()
    }

    /// Returns a map of parameter names to their parameter data.
    #[must_use]
    pub fn parameters(&self) -> IndexMap<&'runtime ParameterName, &'runtime ir::Parameter> {
        self.model.get_parameters().iter().collect()
    }

    /// Returns a parameter by its name.
    #[must_use]
    pub fn get_parameter(&self, name: &ParameterName) -> Option<&'runtime ir::Parameter> {
        self.model.get_parameters().get(name)
    }

    /// Returns the list of tests for this model.
    #[must_use]
    pub const fn tests(&self) -> &'runtime IndexMap<TestIndex, ir::Test> {
        self.model.get_tests()
    }

    /// Returns the Python imports for this model.
    #[must_use]
    pub const fn python_imports(&self) -> &'runtime IndexMap<PythonPath, ir::PythonImport> {
        self.model.get_python_imports()
    }
}

/// A reference to a submodel import within a model.
#[derive(Debug, Clone, Copy)]
pub struct SubmodelImportReference<'runtime> {
    submodel_import: &'runtime ir::SubmodelImport,
    ir_cache: &'runtime IrCache,
    references: &'runtime IndexMap<ReferenceName, ir::ReferenceImport>,
}

impl<'runtime> SubmodelImportReference<'runtime> {
    /// Creates a new `SubmodelImportReference` for the given submodel import and IR cache.
    #[must_use]
    pub const fn new(
        submodel_import: &'runtime ir::SubmodelImport,
        ir_cache: &'runtime IrCache,
        references: &'runtime IndexMap<ReferenceName, ir::ReferenceImport>,
    ) -> Self {
        Self {
            submodel_import,
            ir_cache,
            references,
        }
    }

    /// Returns the name of the submodel.
    #[must_use]
    pub const fn name(&self) -> &'runtime SubmodelName {
        self.submodel_import.name()
    }

    /// Returns the span of the name of the submodel.
    #[must_use]
    pub const fn name_span(&self) -> &'runtime Span {
        self.submodel_import.name_span()
    }

    /// Returns the reference name of the submodel.
    #[must_use]
    pub const fn reference_name(&self) -> &'runtime ReferenceName {
        self.submodel_import.reference_name()
    }

    /// Returns the reference import of the submodel.
    #[expect(
        clippy::missing_panics_doc,
        reason = "the panic only happens if an internal invariant is violated"
    )]
    #[must_use]
    pub fn reference_import(&self) -> ReferenceImportReference<'runtime> {
        let reference_name = self.submodel_import.reference_name();
        let reference_import = self
            .references
            .get(reference_name)
            .expect("reference should be found");

        ReferenceImportReference::new(reference_import, self.ir_cache)
    }
}

/// A reference to a reference import within a model.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceImportReference<'runtime> {
    reference_import: &'runtime ir::ReferenceImport,
    ir_cache: &'runtime IrCache,
}

impl<'runtime> ReferenceImportReference<'runtime> {
    /// Creates a new `ReferenceImportReference` for the given reference import and IR cache.
    #[must_use]
    pub const fn new(
        reference_import: &'runtime ir::ReferenceImport,
        ir_cache: &'runtime IrCache,
    ) -> Self {
        Self {
            reference_import,
            ir_cache,
        }
    }

    /// Returns the name of the reference.
    #[must_use]
    pub const fn name(&self) -> &'runtime ReferenceName {
        self.reference_import.name()
    }

    /// Returns the span of the name of the reference.
    #[must_use]
    pub const fn name_span(&self) -> &'runtime Span {
        self.reference_import.name_span()
    }

    /// Returns the path of the reference.
    #[must_use]
    pub const fn path(&self) -> &'runtime ModelPath {
        self.reference_import.path()
    }

    /// Returns the model that this reference imports. If the referenced model
    /// failed to resolve, returns `None`.
    #[expect(
        clippy::missing_panics_doc,
        reason = "the panic only happens if an internal invariant is violated"
    )]
    #[must_use]
    pub fn model(&self) -> Option<ModelIrReference<'runtime>> {
        let entry = self
            .ir_cache
            .get_entry(self.reference_import.path())
            .expect("reference should be in cache");

        let ir = entry.value()?;

        Some(ModelIrReference::new(ir, self.ir_cache))
    }
}
