# Design Overlays: Implementation Guide

This document describes the **implementation architecture** for design files,
overlays, and parameter augmentation in the Rust Oneil implementation. It
complements `designs-and-imports.md` (the user-facing spec) with developer-
focused details.

## Overview

Design files enable parameterization and composition of models through three
mechanisms:

1. **Parameter overrides** — replace a parameter's RHS at evaluation time.
2. **Parameter additions** — introduce new parameters that don't exist on the
   target model (also reachable as `new_param.ref` from an enclosing model).
3. **Reference replacements** — swap the model bound to an alias.

Pipeline at a glance:

```
parse → resolve (per-file IR) → build InstanceGraph → eval
```

The resolver's job is to produce a clean per-file IR (`Model`, `Design`,
`DesignApplication`). It does **not** stamp out per-instance overlays or walk
reference chains. Composition happens once, in the **instance graph build
pass**, which is consumed by evaluation.

## Data Structures

### Per-file IR: `Design` and `DesignApplication`

```rust
// oneil_ir/src/design_overlay.rs
pub struct Design {
    pub target_model: Option<ModelPath>,
    pub parameter_overrides: IndexMap<ParameterName, OverlayParameterValue>,
    pub scoped_overrides:
        IndexMap<InstancePath, IndexMap<ParameterName, OverlayParameterValue>>,
    pub reference_replacements: IndexMap<ReferenceName, ReferenceReplacement>,
    pub parameter_additions: IndexMap<ParameterName, Parameter>,
}

pub struct DesignApplication {
    pub design: Design,             // resolved design content
    pub for_reference: Option<ReferenceName>, // None ⇒ apply to self
    pub design_span: Span,
}
```

- `Design` is the resolved, declarative form of a `.one` design file.
- A `Model` carries `applied_designs: Vec<DesignApplication>` — one entry per
  `use design X [for ref]` line in the file. The instancing pass consumes
  these.
- A `Model` also carries `augmented_reference_params: IndexMap<ReferenceName,
  Design>`, used by the resolver to validate `ref.new_param` lookups against
  parameters introduced by a design applied to that reference.

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
    /// Instance whose lexical scope owns this overlay (where the design landed).
    pub anchor_key: EvalInstanceKey,
}
```

The graph is the **single source of truth** for the live model tree. Every
contribution from a design — overrides, additions, reference replacements —
has been resolved to absolute coordinates by the time the graph is built.

## Resolution Phase

Resolution produces per-file IR. It is intentionally *flat*: it never composes
models with each other.

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. Parse design file (e.g. augment.one)                             │
│    design target                                                    │
│    radius = 10                                                      │
│    diameter = 2 * radius      # new parameter (addition)            │
│                                                                     │
│ 2. resolve_design_surface() processes declarations:                 │
│    a. Find design target, collect parameter names.                  │
│    b. Register design-local params in a scratch table on the        │
│       resolution context so they can cross-reference each other.    │
│    c. For each parameter:                                           │
│       - exists on target → parameter_overrides (override)           │
│       - new                → parameter_additions (addition)         │
│    d. Store the result via Model::set_design_export().              │
│                                                                     │
│ 3. For each `use design X [for ref]` in a model file:               │
│    a. Push a DesignApplication onto Model::applied_designs.         │
│    b. If for-ref and X has parameter_additions, also record         │
│       Model::augmented_reference_params[ref] = X so the resolver    │
│       can validate `ref.new_param` lookups.                         │
└─────────────────────────────────────────────────────────────────────┘
```

The resolver does *not* expand applications into per-instance overlay
records, walk reference chains, or merge designs across files.

### Variable resolution for augmented references

When resolving `ref.new_param` in a model file, the resolver checks both:

1. The referenced model's IR parameters, and
2. `Model::augmented_reference_params[ref].parameter_additions`.

```rust
// oneil_resolver/src/resolver/resolve_parameter.rs
let exists_on_model = model.get_parameter(&var_identifier).is_some();
let is_augmented = resolution_context
    .get_augmented_param_for_reference(&reference_name, &var_identifier)
    .is_some();
if !exists_on_model && !is_augmented {
    return Err(VariableResolutionError::undefined_parameter_in_reference(...));
}
```

## Instance Graph Build Pass

Before evaluation runs, `InstanceGraph::build` walks the model tree from the
root and stamps out one `InstancedModel` per `EvalInstanceKey`. This is the
only pass that performs design composition.

```
┌─────────────────────────────────────────────────────────────────────┐
│ InstanceGraph::build(root_path, runtime_designs, external):         │
│                                                                     │
│   visit(root_key, landed=runtime_designs):                          │
│     1. Load IR for the model at this key.                           │
│     2. Append this model's own DesignApplications to `landed`.      │
│        Each contribution carries its anchor_key — the instance      │
│        where the design landed (its lexical scope).                 │
│     3. Compute reference replacements: combine the model's own      │
│        references with replacements supplied by any landed design.  │
│     4. Build the InstancedModel:                                    │
│        - parameters = IR + parameter_additions from landed designs  │
│        - references = post-replacement child keys                   │
│        - overlays   = parameter_overrides from landed designs,      │
│                      anchored at the contributing design's scope    │
│        - tests      = IR tests                                      │
│     5. For each child reference, build a forwarded contribution     │
│        list (extracting matching `scoped_overrides` segments) and   │
│        recurse with that as `landed`.                               │
└─────────────────────────────────────────────────────────────────────┘
```

The `anchor_key` is the central trick that makes overlays evaluate in the
correct lexical scope. When a design lands at instance `A` but the override
targets a parameter on a deeper instance `A.b.c`, the override's RHS is
written in `A`'s scope (it can mention `A`'s parameters). We must remember
that scope so the evaluator can switch into it.

### Runtime designs vs. file-level designs

The CLI's `--design path.one` is implemented by injecting a single
`DesignApplication` (with `for_reference = None`) onto the root before the
graph is built:

```rust
// oneil_runtime/src/runtime/eval.rs
let runtime_designs = match design_path {
    Some(path) => {
        let design = load_design(path);
        vec![ir::DesignApplication {
            design,
            for_reference: None,
            design_span: Span::empty(...),
        }]
    }
    None => Vec::new(),
};
let graph = InstanceGraph::build(root_path, &runtime_designs, &external);
```

There is no second code path for "with design" vs "without design" — runtime
designs flow through exactly the same instancing pass as `use design`
declarations inside model files.

## Evaluation Phase

Evaluation uses a **lazy memoized** strategy. `EvalContext` is seeded from
the `InstanceGraph` (every parameter starts as `Pending`, every reference is
already wired). Then we force every parameter; cycles surface naturally on
re-entry.

```rust
// oneil_eval/src/eval_model.rs
pub fn eval_model_with_designs(...) -> ... {
    let graph = InstanceGraph::build(model_path, runtime_designs, external);
    eval_model_from_graph(&graph, external)
}

pub fn eval_model_from_graph(graph: &InstanceGraph, external) -> ... {
    let mut context = EvalContext::from_graph(graph, external);
    force_all_models(&mut context);
    propagate_reference_errors(&mut context);
    collect_results(&context)
}
```

### Memo table: `ParamSlot`

Each parameter starts as `Pending(ir::Parameter)` in the memo table:

| State | Meaning |
|-------|---------|
| `Pending(ir::Parameter)` | Not yet evaluated; holds the IR to evaluate it |
| `InProgress` | Currently being evaluated; re-entry means a cycle |
| `Done(Result<output::Parameter, …>)` | Evaluation complete |

Transition: `Pending → InProgress → Done`.

### `force_parameter` — the lazy core

When any expression or external lookup needs a parameter value, it calls
`EvalContext::force_parameter(key, name)`:

```
1. Peek at the slot for (key, name):
   - Done            → return the cached result immediately
   - InProgress      → return EvalError::CircularParameterEvaluation
   - TakenForEval(ir) → swap the slot to InProgress, eval, then write Done
2. Push `key` as the active model (eval scope).
3. Call eval_parameter() with &mut EvalContext.
4. Pop the active model.
5. Write the Done result and return it.
```

Cycle detection is zero-cost at steady state: cycles surface as
`EvalError::CircularParameterEvaluation` on the first re-entrant lookup.

### Overlay evaluation: anchor scope

When `eval_parameter` finds an overlay for the parameter being forced, it
evaluates the overlay's RHS in the **anchor's** scope (the instance whose
design contributed the override), not the target's scope:

```
1. instanced.overlays.get(&name) → OverlayBinding { value, anchor_key, … }
2. If anchor_key ≠ current active model key:
   a. Push anchor_key as active model
   b. Evaluate the overlay expression
   c. Pop anchor_key
3. Otherwise evaluate in the already-active scope.
```

This is what makes `parent.on` writing `child.thing = 2 * multiplier` resolve
`multiplier` against `parent`'s parameters even though the override lands on
the `child` instance.

### Key types and functions

| Item | Location | Purpose |
|------|----------|---------|
| `InstanceGraph::build` | `instance_graph.rs` | Compose designs into instances |
| `InstancedModel`       | `instance_graph.rs` | Per-instance composed view |
| `OverlayBinding`       | `instance_graph.rs` | Resolved override + anchor scope |
| `EvalContext::from_graph` | `context.rs`     | Seed context from the graph |
| `eval_model_with_designs` | `eval_model.rs`  | Build graph + drive evaluation |
| `eval_model_from_graph`   | `eval_model.rs`  | Drive evaluation from a built graph |
| `force_parameter`         | `context.rs`     | Lazy memo-table core |
| `eval_parameter`          | `eval_parameter.rs` | Single-parameter evaluation |

### The `eval_scope` stack

`EvalContext::eval_scope: Vec<EvalInstanceKey>` is the dynamic stack of
"who's evaluating right now". `.last()` is always the model whose
parameters/overlays are currently being evaluated. It is pushed by:

- `force_parameter` — for the duration of evaluating a parameter in its own
  model's scope.
- The overlay-anchor bracket in `eval_parameter` — for the duration of
  evaluating an overlay's RHS in the design's lexical scope.

After graph construction, the stack is *not* an "ancestor chain"; the graph
already holds all reference wiring. The stack only tracks the dynamic
evaluation focus.

## Design-Augmented References (`new_param.ref`)

When a model applies a design to a reference (`use design D for b`), and `D`
adds new parameters (`parameter_additions`), those parameters become
accessible via `new_param.b` (Oneil uses `param.ref` syntax).

The mechanics:

| Phase | What happens |
|-------|--------------|
| Resolution | `Model::augmented_reference_params[b] = D` so `b.new_param` validates |
| Instance graph | `parameter_additions` from `D` land on `b`'s `InstancedModel.parameters` |
| Evaluation | `new_param.b` follows the standard external-lookup path; `b` already has the parameter registered as `Pending` |

### Scoped overrides for augmented parameters

The same scoped-override syntax (`new_param.b = …`) works for design-added
parameters because `parameter_overrides` and `scoped_overrides` are matched
by parameter name, not by IR provenance — the instancing pass doesn't
distinguish "added by a design" from "originally on the target".

## Why a single instancing pass?

Earlier iterations of this code split composition across three places:

- The resolver stamped per-instance overlay records.
- The runtime walked reference chains for `--design`.
- Pass 1 of evaluation re-walked the tree to wire everything together.

That split was fragile (each path had its own subtle ordering) and produced
duplicate concepts (`DeclaredParameterOverlay` vs. `OverlayBinding`).

Funnelling everything through `InstanceGraph::build` gives us one pass with
one mental model: walk the tree, accumulate landed contributions, stamp out
the composed instance, recurse. The resolver only needs to know per-file
shape; the evaluator only needs to know how to consume a fully-composed
graph.

## Resolution-Time Scratch for Design-Local Parameters

When resolving a design file, design-local parameters (additions and the
RHS of overrides) are registered in a temporary scratch table on
`ResolutionContext` (`design_local_scratch`) before their expressions are
resolved. This lets design-local parameters reference each other and the
target model's own parameters without polluting the IR model's parameter
list:

```oneil
design target
diameter = 2 * radius          # references the target's own "radius"
circumference = pi * diameter  # references the design-local "diameter"
```

The scratch is keyed by `(ModelPath, ParameterName)` and is checked in
`lookup_parameter_in_active_model` alongside the model's own IR parameters.
After resolution, the validated `ir::Parameter` values are stored in
`Design::parameter_additions` and carried into evaluation through the
instancing pass.
