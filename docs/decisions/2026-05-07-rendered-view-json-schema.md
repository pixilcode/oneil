# Rendered View JSON Schema

## Status

Accepted

## Context

We want a VSCode webview that shows the evaluated instance tree of an Oneil model, including parameter values, original expressions, and documentation notes (as well as which parameters were changed by a design). The webview needs data from two layers: the evaluated output (`oneil_output::Model`) and the IR (`oneil_ir::Parameter`, `ir::Note`) which carries the original expressions and prose.

## Decision

Add a custom LSP request `oneil/instanceTree` that returns a JSON payload describing the rooted instance tree. The payload is built in `oneil_lsp` by combining the evaluated `output::Model` with the IR parameter definitions accessed via the runtime's template reference. Each node carries its identity, evaluated values, original expression strings, notes, child submodel nodes, and reference cross-links.

Serialization is done with `serde` as a direct dependency of `oneil_shared`, `oneil_output`, and `oneil_ir`. The schema is defined by Rust types via `serde` derives and mirrored as TypeScript types in `vscode/webview-ui/src/types/model.ts`.

Bidirectional messaging (e.g. sweep parameter overrides from the webview to the LSP) is deferred; the API will be decided when the feature is needed.

## Consequences

- `serde` becomes a direct (unconditional) dependency of `oneil_shared`, `oneil_output`, and `oneil_ir`; `serde_json` is added to `oneil_lsp`.
- The LSP server runs a full `eval_model` pass for the rendered view (more expensive than the `check_model` path used for diagnostics); this is acceptable because it is user-initiated.
- TypeScript types must be kept in sync with Rust types manually (no code generation for now).
- Future: parameter sweep inputs and diff at the JSON level extend this schema.

## Future Work: Interactive Variable Hovering in Equations

To enable hover tooltips on variable references within rendered KaTeX equations:

1. **Modify `exprToLatex.ts`** to generate custom KaTeX macros like `\href{#param:var_name}{\mathrm{var}}` or define a custom command that wraps variables in spans with data attributes.

2. **Post-process rendered output** after KaTeX renders, use `querySelectorAll('[data-param]')` to attach event listeners or use event delegation on the equation container.

3. **Context mapping** — pass available parameters to the expression renderer so it can resolve what each variable references: local parameter, external reference (`model.param`), or builtin constant.

4. **Tooltip content** — show the referenced parameter's label, current value, unit, and optionally its note. For external references, also show the source model path.