//! Custom LSP request handlers for the Oneil rendered view.
//!
//! Implements `oneil/instanceTree` (via `workspace/executeCommand`).
//!
//! ## Architecture
//!
//! Evaluation and IR data come from the same `Runtime` but cannot be accessed
//! simultaneously (Rust borrow rules), so work is split into two phases:
//!
//! 1. **Eval phase** — call `runtime.eval_model()`, walk the returned
//!    `ModelReference` tree, clone all evaluated (numeric) data into owned
//!    `EvalNode` structs, then drop the `ModelReference`.
//!
//! 2. **Assembly phase** — with the `ModelReference` borrow released, read
//!    `runtime.composed_graph()`. The composed `InstanceGraph` was built
//!    during evaluation and already carries `ir::Parameter` data (labels,
//!    notes, sections, expression ASTs, design provenance). Walk both trees
//!    in parallel and merge into the final `RenderedNode` tree.
//!
//! This approach is simpler and more correct than calling `load_and_lower`
//! per model path: the composed graph reflects design composition for both
//! `.on` and `.one` files uniformly, while the unit-graph cache (`load_and_lower`)
//! is pre-composition and lacks design provenance for runtime-applied designs.
//!
//! All display/formatting logic (LaTeX, expression pretty-printing) is
//! intentionally deferred to the TypeScript webview.

use std::collections::HashMap;

use oneil_frontend::{InstanceGraph, InstancedModel};
use oneil_runtime::{Runtime, output};
use oneil_shared::paths::ModelPath;
use oneil_shared::symbols::{ParameterName, ReferenceName};
use serde::Serialize;

// ── Response types sent to the webview ───────────────────────────────────────

/// Top-level response containing the main instance tree plus any referenced models.
#[derive(Debug, Serialize)]
pub struct RenderedTree {
    /// The primary model's rendered tree.
    pub root: RenderedNode,
    /// Fully rendered trees for models referenced via `ref` (not `sub`).
    /// These are "external" models linked into the main tree, displayed
    /// separately in the UI's reference pool section.
    pub reference_pool: Vec<RenderedPoolEntry>,
}

/// An entry in the reference pool: a fully rendered model that was referenced.
#[derive(Debug, Serialize)]
pub struct RenderedPoolEntry {
    /// Alias under which this model was first referenced in the main tree.
    pub alias: String,
    /// The fully rendered subtree.
    pub node: RenderedNode,
}

/// One evaluated model instance in the tree.
#[derive(Debug, Serialize)]
pub struct RenderedNode {
    /// Absolute file path of this model.
    pub model_path: String,
    /// Reference-name segments from the evaluation root to this instance.
    pub instance_path: Vec<String>,
    /// Model-level documentation note, if present.
    pub note: Option<String>,
    /// Evaluated parameters in source declaration order.
    pub parameters: Vec<RenderedParameter>,
    /// Submodel children in source declaration order.
    pub children: Vec<RenderedChild>,
    /// Non-submodel `ref` cross-links (no recursive data).
    pub references: Vec<RenderedReference>,
    /// Design files that contributed parameters to this node, in order of
    /// first appearance. The `color_index` can be used to assign a consistent
    /// color from the UI's design palette.
    pub applied_designs: Vec<AppliedDesign>,
}

/// A design file that contributed at least one parameter to a node.
#[derive(Debug, Serialize)]
pub struct AppliedDesign {
    /// Short display name derived from the design file stem (no path, no extension).
    pub design_name: String,
    /// Stable index into the UI's design color palette (0-based, assigned in
    /// order of first appearance across the whole tree).
    pub color_index: usize,
}

/// A submodel child with its fully rendered subtree.
#[derive(Debug, Serialize)]
pub struct RenderedChild {
    /// Alias under which the submodel was declared.
    pub alias: String,
    /// The child's rendered subtree.
    pub node: RenderedNode,
}

/// A non-submodel `ref` cross-link (graph edge only, no subtree).
#[derive(Debug, Serialize)]
pub struct RenderedReference {
    /// Alias under which this reference was declared.
    pub alias: String,
    /// File path of the referenced model.
    pub model_path: String,
}

/// One evaluated parameter with its IR expression and documentation.
#[derive(Debug, Serialize)]
pub struct RenderedParameter {
    /// Source identifier (e.g. `mass`).
    pub name: String,
    /// Human-readable label (e.g. `"Mass"`).
    pub label: String,
    /// Optional section header this parameter was declared under.
    pub section: Option<String>,
    /// Documentation note (from `~ …` in source), if any.
    pub note: Option<String>,
    /// Serialized `ir::ParameterValue` AST.
    ///
    /// The TypeScript side is responsible for rendering this to human-readable
    /// form (plain text, `KaTeX`, etc.). `null` when IR was unavailable.
    pub expression: Option<serde_json::Value>,
    /// Evaluated value.
    pub value: RenderedValue,
    /// Print level: `"none"`, `"trace"`, or `"performance"`.
    pub print_level: &'static str,
    /// Byte offsets of the expression in the source file.
    pub expr_span: ExprSpan,
    /// Design provenance: set when a design contributed this parameter.
    pub design: Option<DesignMark>,
}

/// Records that a design contributed this parameter.
#[derive(Debug, Clone, Serialize)]
pub struct DesignMark {
    /// Short file-stem name of the design (no path or extension).
    pub design_name: String,
    /// `true` if the design *added* this parameter (did not exist on the
    /// base model); `false` if it *overrode* an existing parameter's value.
    pub is_addition: bool,
}

/// Source location of a parameter expression.
#[derive(Debug, Serialize)]
pub struct ExprSpan {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset.
    pub end: usize,
}

/// Serializable evaluated value.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RenderedValue {
    /// A boolean.
    Boolean {
        /// The boolean value.
        value: bool,
    },
    /// A string.
    String {
        /// The string value.
        value: String,
    },
    /// A dimensionless number (scalar or interval).
    Number {
        /// Scalar value, or interval lower bound.
        value: f64,
        /// Interval upper bound, `null` for scalars.
        max: Option<f64>,
    },
    /// A number with a display unit.
    MeasuredNumber {
        /// Scalar value, or interval lower bound (in display unit).
        value: f64,
        /// Interval upper bound, `null` for scalars (in display unit).
        max: Option<f64>,
        /// Display unit string (e.g. `"kg"`, `"m/s^2"`).
        unit: String,
    },
}

// ── Private: phase-1 owned eval data ─────────────────────────────────────────

struct EvalNode {
    model_path_str: String,
    instance_path: Vec<String>,
    parameters: Vec<EvalParam>,
    /// Submodel children (declared with `sub`).
    children: Vec<(String, Self)>,
    /// Reference cross-links (declared with `ref`), now fully evaluated.
    cross_refs: Vec<(String, Self)>,
}

struct EvalParam {
    name: String,
    label: String,
    value: RenderedValue,
    print_level: &'static str,
    expr_span: ExprSpan,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Builds the rendered instance tree for `model_path`.
///
/// Handles both `.on` model files and `.one` design files:
/// - `.on` files are evaluated directly; design provenance comes from `apply`
///   statements baked into the composed graph.
/// - `.one` design files are automatically redirected to their declared target
///   model and evaluated with the design applied; the composed graph carries
///   the provenance.
///
/// Returns a `RenderedTree` containing:
/// - `root`: the main model's tree (submodels nested)
/// - `reference_pool`: fully rendered trees for models referenced via `ref`
///
/// # Errors
///
/// Returns an error string if evaluation fails or the path is invalid.
pub fn build_instance_tree(
    runtime: &mut Runtime,
    model_path: &ModelPath,
) -> Result<RenderedTree, String> {
    // Phase 1: evaluate, collect owned data, release the ModelReference borrow.
    // For .one design files, the runtime automatically redirects to the
    // declared target model and applies the design.
    let eval_node = {
        let (model_opt, _errors) = runtime.eval_model(model_path);
        match model_opt {
            Some(model) => collect_eval_node(model, vec![]),
            None => return Err("evaluation produced no result".to_string()),
        }
    };

    // Phase 2: assemble, reading IR from the composed graph.
    // `composed_graph()` takes &self — safe now that ModelReference is dropped.
    let composed_graph = runtime.composed_graph();
    let mut color_map = HashMap::new();
    let mut reference_pool = Vec::new();
    let root = assemble_node(
        eval_node,
        composed_graph,
        composed_graph.map(|g| g.root.as_ref()),
        &mut color_map,
        &mut reference_pool,
    );
    Ok(RenderedTree {
        root,
        reference_pool,
    })
}

// ── Phase 1 ───────────────────────────────────────────────────────────────────

/// Recursively collects owned evaluated data from a `ModelReference`.
///
/// `instance_path` is the list of alias segments from the evaluation root to
/// this node. The root is called with `vec![]`; each child extends the
/// parent's path with its own alias so every node has a unique ID.
fn collect_eval_node(
    model: output::reference::ModelReference<'_>,
    instance_path: Vec<String>,
) -> EvalNode {
    let model_path_str = model.path().as_path().display().to_string();
    let submodels = model.submodels();

    let parameters = model
        .parameters()
        .into_values()
        .map(collect_eval_param)
        .collect();

    let mut children = Vec::new();
    let mut cross_refs = Vec::new();

    for (alias, child_model) in model.references() {
        let alias_str = alias.as_str().to_string();
        let mut child_path = instance_path.clone();
        child_path.push(alias_str.clone());
        let child_node = collect_eval_node(child_model, child_path);

        if submodels.contains(alias) {
            children.push((alias_str, child_node));
        } else {
            cross_refs.push((alias_str, child_node));
        }
    }

    EvalNode {
        model_path_str,
        instance_path,
        parameters,
        children,
        cross_refs,
    }
}

/// Clones one evaluated parameter into an owned `EvalParam`.
fn collect_eval_param(param: &output::Parameter) -> EvalParam {
    EvalParam {
        name: param.ident.as_str().to_string(),
        label: param.label.as_str().to_string(),
        value: convert_value(&param.value),
        print_level: match param.print_level {
            output::PrintLevel::None => "none",
            output::PrintLevel::Trace => "trace",
            output::PrintLevel::Performance => "performance",
        },
        expr_span: ExprSpan {
            start: param.expr_span.start().offset,
            end: param.expr_span.end().offset,
        },
    }
}

/// Converts an `output::Value` to a serializable `RenderedValue`.
fn convert_value(value: &output::Value) -> RenderedValue {
    match value {
        output::Value::Boolean(b) => RenderedValue::Boolean { value: *b },
        output::Value::String(s) => RenderedValue::String { value: s.clone() },
        output::Value::Number(n) => match n {
            output::Number::Scalar(v) => RenderedValue::Number {
                value: *v,
                max: None,
            },
            output::Number::Interval(i) => RenderedValue::Number {
                value: i.min(),
                max: Some(i.max()),
            },
        },
        output::Value::MeasuredNumber(mn) => {
            let (number, unit) = mn.clone().into_number_and_unit();
            let unit_str = format!("{unit}");
            match number {
                output::Number::Scalar(v) => RenderedValue::MeasuredNumber {
                    value: v,
                    max: None,
                    unit: unit_str,
                },
                output::Number::Interval(i) => RenderedValue::MeasuredNumber {
                    value: i.min(),
                    max: Some(i.max()),
                    unit: unit_str,
                },
            }
        }
    }
}

// ── Phase 2 ───────────────────────────────────────────────────────────────────

/// Returns the file-stem display name for a `ModelPath` (strips the directory
/// path and any `.on` / `.one` extension).
///
/// ```text
/// /path/to/vehicle.on   → "vehicle"
/// /path/to/overlay.one  → "overlay"
/// ```
fn path_stem(path: &ModelPath) -> String {
    path.as_path()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string()
}

/// Merges one `EvalNode` with its corresponding `InstancedModel` from the
/// composed graph, producing a `RenderedNode`.
///
/// `ir_model` is `None` when the composed graph is unavailable or when an
/// alias import has no direct `InstancedModel` entry (graceful fallback — IR
/// fields will be `null` in the JSON output).
///
/// `color_map` accumulates design-name → `color_index` assignments so the
/// same design always gets the same color across the whole tree.
///
/// `out_reference_pool` collects fully rendered trees for `ref` cross-links,
/// displayed separately from the main tree in the UI.
fn assemble_node(
    eval_node: EvalNode,
    composed_graph: Option<&InstanceGraph>,
    ir_model: Option<&InstancedModel>,
    color_map: &mut HashMap<String, usize>,
    out_reference_pool: &mut Vec<RenderedPoolEntry>,
) -> RenderedNode {
    let model_note = ir_model
        .and_then(|m| m.note())
        .map(|n| n.content().to_string());

    let parameters: Vec<RenderedParameter> = eval_node
        .parameters
        .into_iter()
        .map(|ep| {
            let ir_param =
                ir_model.and_then(|m| m.parameters().get(&ParameterName::from(ep.name.as_str())));

            let design = ir_param
                .and_then(|p| p.design_provenance())
                .map(|prov| DesignMark {
                    design_name: path_stem(&prov.design_path),
                    is_addition: prov.is_addition,
                });

            RenderedParameter {
                note: ir_param
                    .and_then(|p| p.note())
                    .map(|n| n.content().to_string()),
                expression: ir_param.and_then(|p| serde_json::to_value(p.value()).ok()),
                section: ir_param
                    .and_then(|p| p.section_label())
                    .map(|s| s.as_str().to_string()),
                name: ep.name,
                label: ep.label,
                value: ep.value,
                print_level: ep.print_level,
                expr_span: ep.expr_span,
                design,
            }
        })
        .collect();

    // Collect designs that appear in this node's parameters, assigning stable
    // colour indices that persist across the whole tree.
    let mut seen_designs: Vec<String> = Vec::new();
    for p in &parameters {
        if let Some(mark) = &p.design
            && !seen_designs.contains(&mark.design_name)
        {
            seen_designs.push(mark.design_name.clone());
        }
    }
    let applied_designs: Vec<AppliedDesign> = seen_designs
        .into_iter()
        .map(|name| {
            let next_idx = color_map.len();
            let color_index = *color_map.entry(name.clone()).or_insert(next_idx);
            AppliedDesign {
                design_name: name,
                color_index,
            }
        })
        .collect();

    // Walk submodel children: look each alias up in the composed graph.
    let children = eval_node
        .children
        .into_iter()
        .map(|(alias, child_eval)| {
            let child_ir = ir_model
                .and_then(|m| m.submodels().get(&ReferenceName::from(alias.as_str())))
                .map(|s| s.instance.as_ref());
            RenderedChild {
                alias,
                node: assemble_node(
                    child_eval,
                    composed_graph,
                    child_ir,
                    color_map,
                    out_reference_pool,
                ),
            }
        })
        .collect();

    // Assemble cross-refs as full nodes and add to the output reference pool.
    // Also keep basic pointers in the node's `references` for edge rendering.
    let mut references = Vec::new();
    for (alias, ref_eval) in eval_node.cross_refs {
        let model_path = ref_eval.model_path_str.clone();
        // Look up IR for this reference from the graph's reference_pool (keyed by ModelPath).
        let ref_ir = composed_graph.and_then(|g| {
            let mp = ModelPath::try_from(model_path.as_str()).ok()?;
            g.reference_pool.get(&mp).map(AsRef::as_ref)
        });
        let ref_node = assemble_node(
            ref_eval,
            composed_graph,
            ref_ir,
            color_map,
            out_reference_pool,
        );

        // Add to the top-level output reference pool.
        out_reference_pool.push(RenderedPoolEntry {
            alias: alias.clone(),
            node: ref_node,
        });

        // Keep a basic pointer in this node for edge rendering.
        references.push(RenderedReference { alias, model_path });
    }

    RenderedNode {
        model_path: eval_node.model_path_str,
        instance_path: eval_node.instance_path,
        note: model_note,
        parameters,
        children,
        references,
        applied_designs,
    }
}
