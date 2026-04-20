# Designs, overlays, and imports

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
- Parameter lines use the **shorthand** form `id = expr` (optional `: unit`), without the full preamble (`Label:` …) used in ordinary model files. Metadata (limits, display name, etc.) comes from the target model's definition.
- **Scoped overrides:** use `param.ref = value` to override a parameter on a nested reference instance (e.g., `thrust.main_engine = 500 :N` overrides `thrust` on the instance bound to `main_engine`).
- Parameters set in a design file that don't exist on the target model are introduced as new parameters on the target at evaluation time (see [parameter additions](#parameter-additions)).

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

Multiple `use design` lines stack; **later** entries win for the same parameter.

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

## Parameter additions

A design file may introduce parameters that do not exist on the target model:

```oneil
design cylinder

radius = 3 :m
diameter = 2 * radius       # new — not on cylinder.on
circumference = pi * diameter
```

These parameters are added to the target instance at evaluation time and are accessible from an enclosing model via `new_param.ref` syntax.

## Submodel replacement vs design overlay

- `use other_model as net` binds the alias `net` to another model file (submodel / reference replacement, when the enclosing model defines `net`).
- `use design foo for net` keeps the existing `net` binding and applies the design file `foo` to that **instance** at evaluation time (eval-time overlay).

## Restrictions

- **No conflicting definitions:** You cannot both define a reference (`ref X` or `use Y as X`) and replace it (`use Z as X`) in the same design file. Reference replacements target references defined in the **target model**, not in the design file itself.

## Evaluation model

- Resolved IR keeps one structure per on-disk model file.
- Before evaluation, an **instance graph** is built that walks the model tree once and stamps out one composed instance per `(model_path, instance_path)`. Design composition — overrides, parameter additions, and reference replacements — is performed exclusively in that build pass.
- **Evaluation** drives the graph lazily: each parameter starts pending in a memo table and is forced on demand, with cycle detection on re-entrance. External references `alias.param` use `alias` to look up the correct child instance, so the same file imported under two aliases yields two different evaluated results when overlays differ.

See [`../architecture/design-overlays.md`](../architecture/design-overlays.md) for the
developer-facing implementation guide, and [grammar.ebnf](grammar.ebnf) for the formal
syntax.
