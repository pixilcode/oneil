use std::path::PathBuf;

use indexmap::IndexMap;

use oneil_ir as ir;
use oneil_shared::{
    labels::ParameterLabel,
    paths::{ModelPath, PythonPath},
    span::Span,
    symbols::{ParameterName, ReferenceName, SubmodelName, TestIndex},
};

/// Generates a span for testing purposes
///
/// The span is intentionally random in order to discourage any
/// use of the spans for testing.
fn unimportant_span() -> Span {
    Span::random_span()
}

/// Generates a model path for testing purposes
///
/// The path is intentionally random in order to discourage any
/// use of the path for testing.
fn unimportant_model_path() -> ModelPath {
    let path = PathBuf::from("unimportant.on");
    ModelPath::from_path_with_ext(&path)
}

// SIMPLE CONSTRUCTORS

pub fn reference_name(reference_name: &str) -> ReferenceName {
    ReferenceName::new(reference_name.to_string())
}

pub fn expr_literal_number(value: f64) -> ir::Expr {
    let span = unimportant_span();
    ir::Expr::literal(span, ir::Literal::number(value))
}

pub fn empty_model() -> ir::Model {
    ir::Model::new(
        unimportant_model_path(),
        IndexMap::new(),
        IndexMap::new(),
        IndexMap::new(),
        IndexMap::new(),
        IndexMap::new(),
        None,
    )
}

// BUILDERS

pub struct ModelBuilder {
    python_imports: IndexMap<PythonPath, ir::PythonImport>,
    submodels: IndexMap<SubmodelName, ir::SubmodelImport>,
    references: IndexMap<ReferenceName, ir::ReferenceImport>,
    parameters: IndexMap<ParameterName, ir::Parameter>,
    tests: IndexMap<TestIndex, ir::Test>,
}

impl ModelBuilder {
    pub fn new() -> Self {
        Self {
            python_imports: IndexMap::new(),
            submodels: IndexMap::new(),
            references: IndexMap::new(),
            parameters: IndexMap::new(),
            tests: IndexMap::new(),
        }
    }

    pub fn with_submodel(mut self, submodel_name: &str, submodel_path: &ModelPath) -> Self {
        let submodel_name = SubmodelName::new(submodel_name.to_string());
        let submodel_name_span = unimportant_span();

        // the reference name is the same as the submodel name
        let reference_name = ReferenceName::new(submodel_name.as_str().to_string());
        let reference_path = submodel_path.clone();

        let submodel_import = ir::SubmodelImport::new(
            submodel_name.clone(),
            submodel_name_span,
            reference_name.clone(),
        );

        let reference_import =
            ir::ReferenceImport::new(reference_name.clone(), submodel_name_span, reference_path);

        self.submodels.insert(submodel_name, submodel_import);
        self.references.insert(reference_name, reference_import);
        self
    }

    pub fn with_literal_number_parameter(mut self, ident: &str, value: f64) -> Self {
        let parameter = ParameterBuilder::new()
            .with_name_str(ident)
            .with_simple_number_value(value)
            .build();

        self.parameters
            .insert(ParameterName::from(ident), parameter);

        self
    }

    pub fn build(self) -> ir::Model {
        ir::Model::new(
            unimportant_model_path(),
            self.python_imports,
            self.submodels,
            self.references,
            self.parameters,
            self.tests,
            None,
        )
    }
}

pub struct ParameterBuilder {
    name: Option<ParameterName>,
    name_span: Option<Span>,
    span: Option<Span>,
    value: Option<ir::ParameterValue>,
    limits: Option<ir::Limits>,
    is_performance: bool,
    trace_level: ir::TraceLevel,
}

impl ParameterBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            name_span: None,
            span: None,
            value: None,
            limits: None,
            is_performance: false,
            trace_level: ir::TraceLevel::None,
        }
    }

    pub fn with_name_str(mut self, name: &str) -> Self {
        let name = ParameterName::from(name);
        self.name = Some(name);
        let span = unimportant_span();
        self.name_span = Some(span);
        self.span = Some(span);

        self
    }

    pub fn with_simple_number_value(mut self, value: f64) -> Self {
        let expr = expr_literal_number(value);
        let value = ir::ParameterValue::simple(expr, None);
        self.value = Some(value);

        self
    }

    pub fn build(self) -> ir::Parameter {
        let name = self.name.expect("name must be set");
        let name_span = self.name_span.unwrap_or_else(unimportant_span);
        let span = self.span.unwrap_or_else(unimportant_span);
        let label = ParameterLabel::from(name.as_str());
        let value = self.value.expect("value must be set");
        let limits = self.limits.unwrap_or_default();
        let is_performance = self.is_performance;
        let trace_level = self.trace_level;

        ir::Parameter::new(
            ir::Dependencies::new(),
            name,
            name_span,
            span,
            label,
            None,
            value,
            limits,
            is_performance,
            trace_level,
            None,
        )
    }
}
