# Design Overlays: Implementation Guide

This document describes the implementation architecture for design files, overlays, and
parameter augmentation. It complements
[`../specs/designs-and-imports.md`](../specs/designs-and-imports.md) (the user-facing
spec) with developer-focused details.

## Pipeline overview

```
parse → resolve (per-file IR) → build InstanceGraph → eval
```

The resolver produces a clean per-file IR (`Model`, `Design`, `DesignApplication`). It
does not stamp out per-instance overlays or walk reference chains. Composition happens
once, in the **instance graph build pass**, which is then consumed by evaluation.

## Data structures

### Per-file IR: `Design` and `DesignApplication`

```rust
// oneil_ir/src/design_overlay.rs
pub struct Design {
    pub target_model: Option<ModelPath>,
    pub parameter_overrides: IndexMap<ParameterName, OverlayParameterValue>,
    pub scoped_overrides: IndexMap<InstancePath, IndexMap<ParameterName, OverlayParameterValue>>,
    pub reference_replacements: IndexMap<ReferenceName, ReferenceReplacement>,
    pub parameter_additions: IndexMap<ParameterName, Parameter>,
}

pub struct DesignApplication {
    pub design_path: ModelPath,
    pub applied_to: Option<ReferenceName>, // None ⇒ apply to self
    pub span: Span,
}
```

`Design` is the resolved, declarative form of a `.one` design file. A `Model` carries
`applied_designs: Vec<DesignApplication>` — one entry per `use design X [for ref]` line
— which the instancing pass consumes.

A `Model` also carries `augmented_reference_params: IndexMap<ReferenceName, Design>`,
used by the resolver to validate `ref.new_param` lookups against parameters introduced by
a design applied to that reference.

### Instance graph: `InstanceGraph` and `InstancedModel`

```rust
// oneil_eval/src/instance_graph.rs
pub struct InstanceGraph {
    pub root: EvalInstanceKey,
    pub instances: IndexMap<EvalInstanceKey, InstancedModel>,
}

pub struct InstancedModel {
    pub model_path: ModelPath,
    pub parameters: IndexMap<ParameterName, ir::Parameter>,
    pub references: IndexMap<ReferenceName, EvalInstanceKey>,
    pub submodels: IndexMap<SubmodelName, ReferenceName>,
    pub tests: IndexMap<TestIndex, ir::Test>,
    pub overlays: IndexMap<ParameterName, OverlayBinding>,
}

pub struct OverlayBinding {
    pub value: ir::ParameterValue,
    pub design_span: Span,
    pub original_model_span: Span,
    /// The instance whose lexical scope owns this overlay.
    pub anchor_key: EvalInstanceKey,
}
```

The graph is the single source of truth for the live model tree. Every contribution from
a design — overrides, additions, reference replacements — has been resolved to absolute
coordinates by the time the graph is built.

## Resolution phase

Resolution produces per-file IR without composing models together.

`resolve_design_surface` runs in two logical steps:

1. **Scan** — find the `design <target>` declaration and collect unscoped parameter names.
   Parameters that don't already exist on the target are registered in a temporary scratch
   table on `ResolutionContext` so design-local params can cross-reference each other and
   the target's own parameters.
2. **Dispatch** — for each surface item:
   - `design <target>` — sets `Design::target_model`.
   - `id = expr` — if `id` exists on the target: `Design::parameter_overrides`; otherwise: `Design::parameter_additions`.
   - `id.ref = expr` — `Design::scoped_overrides[ref][id]`.
   - `use model as alias` — `Design::reference_replacements[alias]`.
   - `use design X` / `use design X for ref` — merges the imported `Design` into the running one.

For each `use design X for ref` in a model file, the resolver also pushes a
`DesignApplication` onto `Model::applied_designs` and, if `X` has
`parameter_additions`, records `Model::augmented_reference_params[ref] = X` so that
`ref.new_param` lookups validate correctly.

### Variable resolution for augmented references

When resolving `ref.new_param`, the resolver checks both the referenced model's IR
parameters and `Model::augmented_reference_params[ref].parameter_additions`:

```rust
let exists_on_model = model.get_parameter(&var_identifier).is_some();
let is_augmented = resolution_context
    .get_augmented_param_for_reference(&reference_name, &var_identifier)
    .is_some();
if !exists_on_model && !is_augmented {
    return Err(VariableResolutionError::undefined_parameter_in_reference(...));
}
```

## Instance graph build pass

`InstanceGraph::build` walks the model tree from the root and stamps out one
`InstancedModel` per `EvalInstanceKey`:

```
visit(key, landed_contributions):
  1. Load IR for the model at this key.
  2. Append this model's own DesignApplications to `landed`.
     Each contribution carries its anchor_key (the instance where the design landed).
  3. Compose reference replacements from the model's IR plus any landed designs.
  4. Build the InstancedModel:
       parameters = IR params + parameter_additions from landed designs
       references = post-replacement child keys
       overlays   = parameter_overrides from landed designs, anchored at the
                    contributing design's scope
       tests      = IR tests
  5. For each child reference, extract the matching scoped_overrides segments
     and recurse with the forwarded contribution list.
```

The `anchor_key` is how overlays evaluate in the correct lexical scope. When a design
lands at instance `A` but targets a parameter on a deeper instance `A.b.c`, the override
RHS is written in `A`'s scope and must be evaluated there.

### Runtime designs vs. file-level designs

The CLI's `--design path.one` injects a single `DesignApplication` (with `applied_to =
None`) at the root before the graph is built. There is no separate code path for
"with design" vs. "without design" — both flow through the same instancing pass.

## Evaluation phase

Evaluation uses a **lazy memoized** strategy. `EvalContext` is seeded from the
`InstanceGraph` (every parameter starts as `Pending`, every reference is already wired).
Parameters are forced on demand; cycles surface on re-entrance.

### Memo table: `ParamSlot`

| State | Meaning |
|-------|---------|
| `Pending(ir::Parameter)` | Not yet evaluated |
| `InProgress` | Currently evaluating; re-entry means a cycle |
| `Done(Result<…>)` | Evaluation complete |

### `force_parameter`

When any expression needs a value, it calls `EvalContext::force_parameter(key, name)`:

```
1. Check the slot:
   - Done       → return cached result
   - InProgress → return EvalError::CircularParameterEvaluation
   - Pending    → take the IR, swap to InProgress, eval, write Done
2. Push `key` as the active model scope.
3. Call eval_parameter().
4. Pop the active model scope.
5. Write Done and return.
```

### Overlay evaluation: anchor scope

When `eval_parameter` finds an overlay, it evaluates the RHS in the **anchor's** scope,
not the target's scope:

```
1. Look up OverlayBinding { value, anchor_key, … } for the parameter.
2. If anchor_key ≠ current active model:
     push anchor_key, evaluate RHS, pop anchor_key.
3. Otherwise evaluate in the already-active scope.
```

This ensures `child.thing = 2 * multiplier` resolves `multiplier` against the parent's
parameters, because the override was written in the parent's (design's) lexical scope.

### Key types and entry points

| Item | Location | Purpose |
|------|----------|---------|
| `InstanceGraph::build` | `instance_graph.rs` | Compose designs into instances |
| `InstancedModel` | `instance_graph.rs` | Per-instance composed view |
| `OverlayBinding` | `instance_graph.rs` | Resolved override + anchor scope |
| `EvalContext::from_graph` | `context.rs` | Seed context from the graph |
| `eval_model_with_designs` | `eval_model.rs` | Build graph + drive evaluation |
| `force_parameter` | `context.rs` | Lazy memo-table core |
| `eval_parameter` | `eval_parameter.rs` | Single-parameter evaluation |

## Design-augmented references (`new_param.ref`)

When a model applies a design to a reference (`use design D for b`) and `D` adds new
parameters, those parameters become accessible via `b.new_param`.

| Phase | What happens |
|-------|--------------|
| Resolution | `augmented_reference_params[b] = D` so `b.new_param` validates |
| Instance graph | `parameter_additions` from `D` land on `b`'s `InstancedModel.parameters` |
| Evaluation | `b.new_param` follows the standard external-lookup path; `b` already has the parameter registered as `Pending` |

Scoped overrides work for augmented parameters too (`new_param.b = …`) because overrides
are matched by parameter name without regard to whether a parameter originated from the
target model or a design.
