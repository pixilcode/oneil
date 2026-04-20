//! Model structures and collections for the Oneil programming language.

use indexmap::IndexMap;
use oneil_shared::{
    paths::{ModelPath, PythonPath},
    symbols::{ParameterName, ReferenceName, SubmodelName, TestIndex},
};

use crate::{
    Note,
    design_overlay::{Design, DesignApplication},
    model_import::{ReferenceImport, SubmodelImport},
    parameter::Parameter,
    python_import::PythonImport,
    test::Test,
};

/// Represents a single Oneil model containing parameters, tests, submodels, and imports.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    path: ModelPath,
    python_imports: IndexMap<PythonPath, PythonImport>,
    submodels: IndexMap<SubmodelName, SubmodelImport>,
    references: IndexMap<ReferenceName, ReferenceImport>,
    parameters: IndexMap<ParameterName, Parameter>,
    tests: IndexMap<TestIndex, Test>,
    note: Option<Note>,
    /// `design <model>` target for this file, when present.
    design_target: Option<ModelPath>,
    /// Resolved design content exported by this file (for `use design` consumers).
    design_export: Design,
    /// Designs applied to references via `use design X for ref`.
    /// Maps reference alias → design containing `parameter_additions` that augment the ref.
    /// Used during resolution to support `ref.augmented_param` lookups.
    augmented_reference_params: IndexMap<ReferenceName, Design>,
    /// Designs applied by this model file via `use design X [for ref]`.
    ///
    /// Declarative records consumed by the instancing pass to stamp overrides,
    /// reference replacements, and parameter additions onto the live tree.
    applied_designs: Vec<DesignApplication>,
}

impl Model {
    /// Creates a new model with the specified components.
    #[must_use]
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        path: ModelPath,
        python_imports: IndexMap<PythonPath, PythonImport>,
        submodels: IndexMap<SubmodelName, SubmodelImport>,
        references: IndexMap<ReferenceName, ReferenceImport>,
        parameters: IndexMap<ParameterName, Parameter>,
        tests: IndexMap<TestIndex, Test>,
        note: Option<Note>,
        design_target: Option<ModelPath>,
        design_export: Design,
    ) -> Self {
        Self {
            path,
            python_imports,
            submodels,
            references,
            parameters,
            tests,
            note,
            design_target,
            design_export,
            augmented_reference_params: IndexMap::new(),
            applied_designs: Vec::new(),
        }
    }

    /// Returns the path of this model.
    #[must_use]
    pub const fn get_path(&self) -> &ModelPath {
        &self.path
    }

    /// Returns a reference to the set of Python imports for this model.
    #[must_use]
    pub const fn get_python_imports(&self) -> &IndexMap<PythonPath, PythonImport> {
        &self.python_imports
    }

    /// Looks up a submodel by its identifier.
    #[must_use]
    pub fn get_submodel(&self, identifier: &SubmodelName) -> Option<&SubmodelImport> {
        self.submodels.get(identifier)
    }

    /// Returns the reference that a submodel is associated with.
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "the panic is only caused by breaking an internal invariant"
    )]
    pub fn get_submodel_reference(&self, identifier: &SubmodelName) -> Option<&ReferenceImport> {
        let submodel = self.submodels.get(identifier)?;
        let reference: &ReferenceImport = self
            .references
            .get(submodel.reference_name())
            .expect("reference corresponding to submodel should exist");
        Some(reference)
    }

    /// Returns a reference to all submodels in this model.
    #[must_use]
    pub const fn get_submodels(&self) -> &IndexMap<SubmodelName, SubmodelImport> {
        &self.submodels
    }

    /// Looks up a parameter by its identifier.
    #[must_use]
    pub fn get_parameter(&self, identifier: &ParameterName) -> Option<&Parameter> {
        self.parameters.get(identifier)
    }

    /// Returns a reference to all parameters in this model.
    #[must_use]
    pub const fn get_parameters(&self) -> &IndexMap<ParameterName, Parameter> {
        &self.parameters
    }

    /// Looks up a reference by its identifier.
    #[must_use]
    pub fn get_reference(&self, identifier: &ReferenceName) -> Option<&ReferenceImport> {
        self.references.get(identifier)
    }

    /// Returns a reference to all references in this model.
    #[must_use]
    pub const fn get_references(&self) -> &IndexMap<ReferenceName, ReferenceImport> {
        &self.references
    }

    /// Returns a reference to all tests in this model.
    #[must_use]
    pub const fn get_tests(&self) -> &IndexMap<TestIndex, Test> {
        &self.tests
    }

    /// Returns the optional note attached to this model.
    #[must_use]
    pub const fn note(&self) -> Option<&Note> {
        self.note.as_ref()
    }

    /// Returns the declared `design <model>` target path, if any.
    #[must_use]
    pub const fn design_target(&self) -> Option<&ModelPath> {
        self.design_target.as_ref()
    }

    /// Returns the resolved design content this file exports.
    #[must_use]
    pub const fn design_export(&self) -> &Design {
        &self.design_export
    }

    /// Returns a mutable reference to the design content this file exports.
    #[must_use]
    pub const fn design_export_mut(&mut self) -> &mut Design {
        &mut self.design_export
    }

    /// Sets the `design` target for this model file.
    pub fn set_design_target(&mut self, target: Option<ModelPath>) {
        self.design_target = target;
    }

    /// Sets the exported design content for this model file.
    pub fn set_design_export(&mut self, design: Design) {
        self.design_export = design;
    }

    /// Returns the designs applied to references via `use design X for ref`.
    #[must_use]
    pub const fn augmented_reference_params(&self) -> &IndexMap<ReferenceName, Design> {
        &self.augmented_reference_params
    }

    /// Returns the design for a specific augmented reference, if any.
    #[must_use]
    pub fn get_augmented_design(&self, reference: &ReferenceName) -> Option<&Design> {
        self.augmented_reference_params.get(reference)
    }

    /// Adds a design that augments a reference with new parameters.
    pub fn add_augmented_reference(&mut self, reference: ReferenceName, design: Design) {
        self.augmented_reference_params.insert(reference, design);
    }

    /// Returns the declarative `use design …` applications recorded for this model.
    #[must_use]
    pub const fn applied_designs(&self) -> &[DesignApplication] {
        self.applied_designs.as_slice()
    }

    /// Records a `use design X [for ref]` application on this model.
    pub fn add_applied_design(&mut self, application: DesignApplication) {
        self.applied_designs.push(application);
    }

    /// Adds a Python import to this model.
    pub fn add_python_import(&mut self, path: PythonPath, import: PythonImport) {
        self.python_imports.insert(path, import);
    }

    /// Adds a reference to this model.
    pub fn add_reference(&mut self, name: ReferenceName, import: ReferenceImport) {
        self.references.insert(name, import);
    }

    /// Adds a submodel to this model.
    pub fn add_submodel(&mut self, name: SubmodelName, import: SubmodelImport) {
        self.submodels.insert(name, import);
    }

    /// Adds a parameter to this model.
    pub fn add_parameter(&mut self, name: ParameterName, parameter: Parameter) {
        self.parameters.insert(name, parameter);
    }

    /// Adds a test to this model.
    pub fn add_test(&mut self, index: TestIndex, test: Test) {
        self.tests.insert(index, test);
    }

    /// Sets the note attached to this model.
    pub fn set_note(&mut self, note: Note) {
        self.note = Some(note);
    }
}
