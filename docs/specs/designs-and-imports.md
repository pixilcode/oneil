# Designs, overlays, and imports (Rust Oneil)

This document describes **what the Rust implementation supports** for design parameterizations and model composition. It is the source of truth for this branch.

## Design files and `design`

A file may declare that it parameterizes another model:

```oneil
design radar
```

The target model can include a directory path:

```oneil
design ../models/satellite
design sensors/radar
```

- The rest of the file is interpreted relative to that **target model** (resolves like a normal model path, relative to this file's location).
- Parameter lines may use the **shorthand** form `id = expr` (optional `: unit`), without the full preamble (`Label:` …) used in ordinary model files. Metadata (limits, display name, etc.) comes from the target model's IR when the design is applied.
- **Scoped overrides:** use `param.ref = value` to override a parameter on a nested reference instance (e.g., `thrust.main_engine = 500 :N` overrides `thrust` on the instance bound to `main_engine`).
- Design files can set parameters that don't exist on the target model; these are silently ignored at evaluation time.

## Direct evaluation of design files

Design files (`.one`) can be evaluated directly since they specify their target model via the `design` declaration. Running `oneil mydesign.one` evaluates the target model with the design applied.

## `use design`

Design files loaded by `use design <path>` use the **`.one`** extension (e.g., `use design foo` loads `foo.one`). Ordinary models remain **`.on`**.

The design file path supports directory prefixes:

```oneil
use design ../designs/network_design for net
use design configs/antenna_design for a
```

Apply a design file to the current design target or to a specific import instance:

```oneil
use design network_design for net
use design antenna_design for a
```

- `for <alias>` selects the reference/submodel **alias** in the current file's import graph.
- Without `for`, the design applies to the file's current **design target** (from `design <model>`):

```oneil
use design balloon_main
```

Multiple `use design` lines stack; **later** entries win for the same parameter (same merge idea as layering design files).

## Reference replacement in design files

In design files (`.one`), you can replace a reference from the target model with a different model:

```oneil
design satellite

use balloon_satellite as sat
use ../models/custom_antenna as antenna with [feed as f]
```

- `use model as alias` replaces the `alias` reference in the target model with `model`.
- The replacement model path supports full directory paths, just like regular `use` imports.
- The optional `with` clause specifies which submodels to extract from the replacement model.

## Scoped parameter overrides

Inside a design file, use dotted syntax to override parameters on nested reference instances:

```oneil
design spacecraft

thrust.main_engine = 1000 :N
thrust.aux_thruster = 200 :N
```

- `thrust` is the parameter name on the referenced model (e.g., a thruster model has a `thrust` parameter).
- `main_engine` is the reference alias in the target model (e.g., `use thruster as main_engine`).
- The value `1000 :N` replaces the evaluated value for the `thrust` parameter on the `main_engine` instance.

This is useful for overriding parameters on submodels without replacing the entire reference.

## Submodel replacement vs design overlay

- `use other_model as net` binds the alias `net` to another model file (submodel / reference replacement, when the enclosing model defines `net`).
- `use design foo for net` keeps the existing `net` binding and applies the design file `foo` to that **instance** at evaluation time (eval-time overlay).

## Restrictions

- **No conflicting definitions:** You cannot both define a reference (`ref X` or `use Y as X`) and replace it (`use Z as X`) in the same design file. Reference replacements target references defined in the **target model**, not in the design file itself.

## Evaluation model

- Resolved IR remains **one structure per on-disk model file**.
- Before evaluation, an **instance graph** is built that walks the model tree
  once and stamps out one composed instance per `(model_path, instance_path)`.
  Design composition — overrides, parameter additions, and reference
  replacements — is performed exclusively in that build pass.
- **Evaluation** then drives the graph lazily: each parameter starts pending
  in a memo table and is forced on demand, with cycle detection by
  re-entrance. External references `alias.param` use `alias` to look up the
  correct child instance, so the same file imported under two aliases can
  yield two different evaluated results when overlays differ.

See `design-overlays-implementation.md` for the developer-facing implementation
guide, and [grammar.ebnf](grammar.ebnf) for the formal syntax updates.
