//! Declaration constructs for the AST

// TODO: rename `Import` to `ImportPython` and `UseModel` to `ImportModel`
use std::{ops::Deref, path::PathBuf};

use oneil_shared::paths::ModelPath;

use crate::{
    naming::{DirectoryNode, IdentifierNode},
    node::Node,
    note::NoteNode,
    parameter::{ParameterNode, ParameterValueNode},
    test::TestNode,
};

/// A declaration in an Oneil program
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    /// Import declaration for including other modules
    Import(ImportNode),

    /// Model usage declaration for referencing other models
    UseModel(UseModelNode),

    /// Declares that this file is a design bundle for another model (`design <name>`).
    DesignTarget(DesignTargetNode),

    /// Applies a design file to the current design target or to a specific import (`use design …`).
    UseDesign(UseDesignNode),

    /// Parameter assignment in a design file (`id = expr`, no label preamble).
    DesignParameter(DesignParameterNode),

    /// Parameter declaration for defining model parameters
    Parameter(ParameterNode),

    /// Test declaration for verifying model behavior
    Test(TestNode),
}

/// A node containing a declaration
pub type DeclNode = Node<Decl>;

impl Decl {
    /// Creates an import declaration
    #[must_use]
    pub const fn import(path: ImportNode) -> Self {
        Self::Import(path)
    }

    /// Creates a model usage declaration
    #[must_use]
    pub const fn use_model(use_model: UseModelNode) -> Self {
        Self::UseModel(use_model)
    }

    /// Creates a design target declaration
    #[must_use]
    pub const fn design_target(node: DesignTargetNode) -> Self {
        Self::DesignTarget(node)
    }

    /// Creates a `use design` declaration
    #[must_use]
    pub const fn use_design(node: UseDesignNode) -> Self {
        Self::UseDesign(node)
    }

    /// Creates a design parameter line
    #[must_use]
    pub const fn design_parameter(node: DesignParameterNode) -> Self {
        Self::DesignParameter(node)
    }

    /// Creates a parameter declaration
    #[must_use]
    pub const fn parameter(parameter: ParameterNode) -> Self {
        Self::Parameter(parameter)
    }

    /// Creates a test declaration
    #[must_use]
    pub const fn test(test: TestNode) -> Self {
        Self::Test(test)
    }
}

/// An import declaration that specifies a module to include
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    path: Node<String>,
}

/// A node containing an import declaration
pub type ImportNode = Node<Import>;

impl Import {
    /// Creates a new import with the given path
    #[must_use]
    pub const fn new(path: Node<String>) -> Self {
        Self { path }
    }

    /// Returns the import path as a string slice
    #[must_use]
    pub const fn path(&self) -> &Node<String> {
        &self.path
    }
}

/// A model usage declaration that references another model
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseModel {
    directory_path: Vec<DirectoryNode>,
    model: ModelInfoNode,
    submodel_list: Option<SubmodelListNode>,
    model_kind: ModelKind,
}

/// A node containing a model usage declaration
pub type UseModelNode = Node<UseModel>;

impl UseModel {
    /// Creates a new model usage declaration
    #[must_use]
    pub const fn new(
        directory_path: Vec<DirectoryNode>,
        model: ModelInfoNode,
        submodel_list: Option<SubmodelListNode>,
        model_kind: ModelKind,
    ) -> Self {
        Self {
            directory_path,
            model,
            submodel_list,
            model_kind,
        }
    }

    /// Returns the directory path for the model usage
    #[must_use]
    pub const fn directory_path(&self) -> &[DirectoryNode] {
        self.directory_path.as_slice()
    }

    /// Returns the model info being used
    #[must_use]
    pub const fn model_info(&self) -> &ModelInfoNode {
        &self.model
    }

    /// Returns the list of submodels being imported
    #[must_use]
    pub const fn imported_submodels(&self) -> Option<&SubmodelListNode> {
        self.submodel_list.as_ref()
    }

    /// Returns the kind of model being used
    #[must_use]
    pub const fn model_kind(&self) -> ModelKind {
        self.model_kind
    }

    /// Returns the relative path of the model
    #[must_use]
    pub fn get_model_relative_path(&self) -> ModelPath {
        let mut path = self
            .directory_path
            .iter()
            .map(|d| d.as_str())
            .collect::<Vec<_>>();
        path.push(self.model.top_component().as_str());

        let path = PathBuf::from(path.join("/"));

        ModelPath::from_path_no_ext(&path)
    }
}

/// A collection of imported model info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    top_component: IdentifierNode,
    subcomponents: Vec<IdentifierNode>,
    alias: Option<IdentifierNode>,
}

/// A node containing model info
pub type ModelInfoNode = Node<ModelInfo>;

impl ModelInfo {
    /// Creates a new model info
    #[must_use]
    pub const fn new(
        top_component: IdentifierNode,
        subcomponents: Vec<IdentifierNode>,
        alias: Option<IdentifierNode>,
    ) -> Self {
        Self {
            top_component,
            subcomponents,
            alias,
        }
    }

    /// Returns the top component of the model
    #[must_use]
    pub const fn top_component(&self) -> &IdentifierNode {
        &self.top_component
    }

    /// Returns the list of subcomponents of the model
    #[must_use]
    pub const fn subcomponents(&self) -> &[IdentifierNode] {
        self.subcomponents.as_slice()
    }

    /// Returns the calculated name of the model
    ///
    /// This is the name of the last subcomponent, or the name of the top
    /// component if there are no subcomponents.
    ///
    /// ## Examples
    ///
    /// ```oneil
    /// # name: `baz`
    /// use foo/bar.baz as qux
    ///
    /// # name: `foo`
    /// ref foo as bar
    ///
    /// # name: `bar`
    /// use foo/bar
    ///
    /// # name: `foo`
    /// ref foo
    /// ```
    #[must_use]
    pub fn get_model_name(&self) -> &IdentifierNode {
        self.subcomponents.last().unwrap_or(&self.top_component)
    }

    /// Returns the calculated alias of the model
    ///
    /// This is the given alias if one is provided. Otherwise, it is the model
    /// name
    ///
    /// ## Examples
    ///
    /// ```oneil
    /// # alias: `qux`
    /// use foo/bar.baz as qux
    ///
    /// # alias: `bar`
    /// ref foo as bar
    ///
    /// # alias: `bar`
    /// use foo/bar
    ///
    /// # alias: `foo`
    /// ref foo
    /// ```
    #[must_use]
    pub fn get_alias(&self) -> &IdentifierNode {
        self.alias.as_ref().unwrap_or_else(|| self.get_model_name())
    }

    /// Returns the explicit alias if one was provided in the declaration.
    ///
    /// This returns `None` if no `as <alias>` was specified.
    #[must_use]
    pub const fn alias(&self) -> Option<&IdentifierNode> {
        self.alias.as_ref()
    }
}

/// A collection of submodel information nodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmodelList(Vec<ModelInfoNode>);

/// A node containing a submodel list
pub type SubmodelListNode = Node<SubmodelList>;

impl SubmodelList {
    /// Creates a new submodel list
    #[must_use]
    pub const fn new(submodel_list: Vec<ModelInfoNode>) -> Self {
        Self(submodel_list)
    }
}

impl Deref for SubmodelList {
    type Target = Vec<ModelInfoNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The kind of model being used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// The model is being used for reference
    Reference,
    /// The model is being used as a submodel
    Submodel,
}

impl ModelKind {
    /// Returns the reference model kind
    #[must_use]
    pub const fn reference() -> Self {
        Self::Reference
    }

    /// Returns the submodel model kind
    #[must_use]
    pub const fn submodel() -> Self {
        Self::Submodel
    }
}

/// Target model path in a `design [path/to/]<name>` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignTarget {
    /// Optional directory path (e.g., `../models/`).
    directory_path: Vec<DirectoryNode>,
    /// The target model name.
    target: IdentifierNode,
}

/// AST node for a [`DesignTarget`].
pub type DesignTargetNode = Node<DesignTarget>;

impl DesignTarget {
    /// Creates a design target declaration with just a model name.
    #[must_use]
    pub const fn new(target: IdentifierNode) -> Self {
        Self {
            directory_path: Vec::new(),
            target,
        }
    }

    /// Creates a design target declaration with a directory path.
    #[must_use]
    pub const fn with_path(directory_path: Vec<DirectoryNode>, target: IdentifierNode) -> Self {
        Self {
            directory_path,
            target,
        }
    }

    /// Returns the directory path for the target model.
    #[must_use]
    pub const fn directory_path(&self) -> &[DirectoryNode] {
        self.directory_path.as_slice()
    }

    /// Returns the target model identifier.
    #[must_use]
    pub const fn target(&self) -> &IdentifierNode {
        &self.target
    }

    /// Returns the relative path of the target model.
    #[must_use]
    pub fn get_target_relative_path(&self) -> ModelPath {
        let mut path = self
            .directory_path
            .iter()
            .map(|d| d.as_str())
            .collect::<Vec<_>>();
        path.push(self.target.as_str());

        let path = PathBuf::from(path.join("/"));

        ModelPath::from_path_no_ext(&path)
    }
}

/// `use design [path/to/]<file> [for <alias>]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDesign {
    /// Optional directory path (e.g., `../designs/`).
    directory_path: Vec<DirectoryNode>,
    /// Design file name (without extension).
    design_file: IdentifierNode,
    /// When `None`, the design applies to this file's [`DesignTarget`].
    instance: Option<IdentifierNode>,
}

/// AST node for a [`UseDesign`].
pub type UseDesignNode = Node<UseDesign>;

impl UseDesign {
    /// Creates a `use design` declaration with just a file name.
    #[must_use]
    pub const fn new(design_file: IdentifierNode, instance: Option<IdentifierNode>) -> Self {
        Self {
            directory_path: Vec::new(),
            design_file,
            instance,
        }
    }

    /// Creates a `use design` declaration with a directory path.
    #[must_use]
    pub const fn with_path(
        directory_path: Vec<DirectoryNode>,
        design_file: IdentifierNode,
        instance: Option<IdentifierNode>,
    ) -> Self {
        Self {
            directory_path,
            design_file,
            instance,
        }
    }

    /// Returns the directory path for the design file.
    #[must_use]
    pub const fn directory_path(&self) -> &[DirectoryNode] {
        self.directory_path.as_slice()
    }

    /// Design file name.
    #[must_use]
    pub const fn design_file(&self) -> &IdentifierNode {
        &self.design_file
    }

    /// Optional `for <alias>` import instance selector.
    #[must_use]
    pub const fn instance(&self) -> Option<&IdentifierNode> {
        self.instance.as_ref()
    }

    /// Returns the relative path of the design file (without extension).
    #[must_use]
    pub fn get_design_relative_path(&self) -> ModelPath {
        let mut path = self
            .directory_path
            .iter()
            .map(|d| d.as_str())
            .collect::<Vec<_>>();
        path.push(self.design_file.as_str());

        let path = PathBuf::from(path.join("/"));

        ModelPath::from_path_no_ext(&path)
    }
}

/// `id = value` or `id.instance = value` line allowed after `design` in design files.
///
/// When `instance` is present, the parameter override applies to a child instance
/// rather than the design target itself. For example, `mass.sat = 5 kg` overrides
/// the `mass` parameter on the `sat` instance.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignParameter {
    ident: IdentifierNode,
    /// Optional instance scope for the parameter override (e.g., `sat` in `mass.sat = 5 kg`).
    instance: Option<IdentifierNode>,
    value: ParameterValueNode,
    note: Option<NoteNode>,
}

/// AST node for a [`DesignParameter`].
pub type DesignParameterNode = Node<DesignParameter>;

impl DesignParameter {
    /// Creates a design parameter line without instance scope.
    #[must_use]
    pub const fn new(
        ident: IdentifierNode,
        value: ParameterValueNode,
        note: Option<NoteNode>,
    ) -> Self {
        Self {
            ident,
            instance: None,
            value,
            note,
        }
    }

    /// Creates a design parameter line with instance scope.
    #[must_use]
    pub const fn with_instance(
        ident: IdentifierNode,
        instance: IdentifierNode,
        value: ParameterValueNode,
        note: Option<NoteNode>,
    ) -> Self {
        Self {
            ident,
            instance: Some(instance),
            value,
            note,
        }
    }

    /// Parameter identifier being assigned.
    #[must_use]
    pub const fn ident(&self) -> &IdentifierNode {
        &self.ident
    }

    /// Optional instance scope for the parameter override.
    #[must_use]
    pub const fn instance(&self) -> Option<&IdentifierNode> {
        self.instance.as_ref()
    }

    /// Assigned value (expression or piecewise).
    #[must_use]
    pub const fn value(&self) -> &ParameterValueNode {
        &self.value
    }

    /// Optional trailing note.
    #[must_use]
    pub const fn note(&self) -> Option<&NoteNode> {
        self.note.as_ref()
    }
}
