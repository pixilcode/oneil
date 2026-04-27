# Designs

A **design file** (extension `.one`) parameterizes a model: it overrides parameter
values, adds new parameters, and can be applied to specific submodel instances.
This lets you explore "what if" scenarios — alternative materials, components, configurations, different
planetary environments — without touching the model
itself.

<!-- outline:
  1. Design files — syntax and running directly
  2. Applying a design from within a model (apply)
  3. reference vs submodel — instance isolation
  4. Modifying submodel parameters directly (param.ref = value)
  5. Adding performance and other parameters (augmentation)
  6. Submodel aliases\
-->

## Design files

A design file starts with a `design <target>` declaration that names the target model
it refines. The rest of the file can optionally use shorthand syntax for parameter definitions: just
`identifier = expression` (and an optional `:unit`), without the full
`Label: id = expr` preamble used in model files. 

```oneil
# mars_gravity.one
design planet

g = 3.72 :m/s^2
R = 3390 :km
```

```oneil
# planet.on
Surface gravity: g = 9.81 :m/s^2
Radius: R = 6371 :km
Mass: M = 5.97e24 :kg
```

Parameters in the design that already exist on the target model are
**overrides**. Parameters that do not exist on the target are
**additions** — useful for adding performance parameters or tests for the specific configuration (see [Adding performance and other parameters](#adding-performance-and-other-parameters)).

### Running a design file directly

Because the design file declares its target model, you can run it directly:

```sh
oneil mars_gravity.one -P all
```

This evaluates `planet.on` with the Mars gravity design applied:

```oneil-eval-output
g = 3.72 : m/s^2  # Surface gravity
R = 3390 : km  # Radius
M = 5.97e24 : kg  # Mass
```

You can also supply a design explicitly when running a model:

```sh
oneil eval planet.on --design mars_gravity.one -P all
```

## Applying a design from within a model

Use `apply <design> to <model_reference>` to wire a design file to a specific submodel
instance inside a model. The design must target the same model that `<ref>`
resolves to or an error will be reported.

```oneil
# mission.on
submodel planet as target
apply mars_gravity to target

Spacecraft mass: m = 500 :kg
$ Surface weight: W = m * g.target :N
```

```sh
oneil eval mission.on
```

```oneil-eval-output
W = 1860 : N  # Surface weight
```

### Applying at the command line

A design can also be applied to the *root* model at the command line — no
`apply` line needed in the model file:

```sh
oneil eval planet.on --design mars_gravity.one -P all
```

## `reference` vs `submodel` — design isolation

How a model is imported determines whether an applied design affects one
instance or all readers.

**`reference`** creates a **shared model**. 
Use this kind of import for a model file that parameterizes the system as a whole, and not an individual model.

<!-- TODO: This seems like it is a property of the file defining the constants and not the import, though I guess what could be a constant for one person could be a parameter for another. -->

**`submodel`** creates a **unique model instance**. 
Two `submodel` imports of the same file are completely independent — a design applied to one does not affect the other.

```oneil
# constants.on
Speed of light: c = 299792458 :m/s
Gravitational constant: G = 6.674e-11 :N*m^2/kg^2
```

```oneil
# mission.on
# Constants are shared — one reference, one instance.
reference constants as phys

# Each planet is an independent instance.
submodel planet as earth
submodel planet as mars
apply mars_gravity to mars

Spacecraft mass: m = 500 :kg
Weight on Earth: W_e = m * g.earth :N   # 4905 N
Weight on Mars:  W_m = m * g.mars  :N   # 1860 N
```

Since `constants` is imported as a `reference` if a design changed `G`, both
`earth` and `mars` (which may use `G` internally as a reference import) would see the updated value.
If `constants` were imported as a `submodel` each copy would be
independent.

> **Rule of thumb:** import shared environmental data (constants, celestial body catalogs, etc) as `reference`. 
> Import components and system elements as `submodel`.

### Sibling designs

Two submodel imports of the same model can each receive a different design:

```oneil
# multi_planet.on
submodel planet as earth
submodel planet as mars
submodel planet as moon

apply mars_gravity to mars
apply moon_gravity to moon

Spacecraft mass: m = 500 :kg
$ Weight on Earth: W_earth = m * g.earth :N
$ Weight on Mars:  W_mars  = m * g.mars  :N
$ Weight on Moon:  W_moon  = m * g.moon  :N
```

```sh
oneil eval multi_planet.on
```

```oneil-eval-output
W_earth = 4905 : N  # Weight on Earth
W_mars = 1860 : N  # Weight on Mars
W_moon = 810 : N  # Weight on Moon
```

## Modifying submodel parameters directly

A design file can override a parameter on a **nested** submodel instance using
dotted syntax: `parameter.ref = value`.

```oneil
# thruster.on
Thrust: thrust = 500 :N
Specific impulse: isp = 300 :s
```

```oneil
# spacecraft.on
submodel thruster as main_engine
submodel thruster as rcs

Thrust scale factor: thrust_scale = 1.0
$ Total thrust: F_total = thrust.main_engine + thrust.rcs :N
```

Default (both thrusters at 500 N):

```sh
oneil eval spacecraft.on
```

```oneil-eval-output
F_total = 1e3 : N  # Total thrust
```

```oneil
# high_thrust.one
design spacecraft

# Override the main engine without touching RCS.
thrust.main_engine = 2000 :N
isp.main_engine    = 450  :s
```

```sh
oneil eval spacecraft.on --design high_thrust.one
```

```oneil-eval-output
F_total = 2500 : N  # Total thrust
```

The RHS of a scoped override is evaluated in the **design's target scope** (the
`spacecraft` model), not inside `main_engine`. This means you can reference
`spacecraft`-level parameters on the right-hand side:

```oneil
# scaled_engine.one
design spacecraft

# thrust_scale is a parameter on spacecraft.on, not on thruster.on.
thrust.main_engine = thrust_scale * thrust.rcs
```

Trying to override a parameter that does not exist on the submodel will result in an error.
If you want to add a parameter to a submodel create a design for the submodel and apply it.

Overriding a parameter with an equation or constant with different dimensions will result in an error.

## Adding performance and other parameters

A design can add parameters that do not exist on the target model. These
new parameters are accessible from an enclosing model as `new_param.ref`.

```oneil
# planet.on
Surface gravity: g = 9.81 :m/s^2
Radius: R = 6371 :km
Mass: M = 5.97e24 :kg
```

```oneil
# derived_planet.one
design planet

# Overrides — exist on planet.on
g = 3.72 :m/s^2
R = 3390 :km

# Augmentations — new parameters not on planet.on
surface_area = 4 * pi * R^2 :km^2
day_length = 24.6 :hr
```

```oneil
# mission_aug.on
submodel planet as target
apply derived_planet to target

$ Day length: t_day = day_length.target :hr
$ Surface area: A = surface_area.target :km^2
```

```sh
oneil eval mission_aug.on
```

```oneil-eval-output
t_day = 24.6 : hr  # Day length
A = 1.444e8 : km^2  # Surface area
```

Augmented parameters can cross-reference other parameters in the design file as
well as parameters already on the target model, and are evaluated alongside all
other parameters.

## Submodel aliases

When you declare a local alias for a nested submodel
(described in [Importing models](./09-importing-models.md)), the local alias
and the one inside the intermediate model refer to the **same instance**.
A design applied through the local alias is therefore visible through the
intermediate model's own reads of that submodel too.

```oneil
# solar_system.on
submodel planet as earth

Earth surface gravity: g_surface = g.earth :m/s^2
```

```oneil
# galaxy.on
submodel solar_system as sol [earth]

Probe mass: m_probe = 800 :kg
$ Landing weight: W = m_probe * g.earth :N
$ Sol gravity reading: g_sol = g_surface.sol :m/s^2
```

Default (Earth gravity on both reads):

```sh
oneil eval galaxy.on
```

```oneil-eval-output
W = 7848 : N  # Landing weight
g_sol = 9.81 : m/s^2  # Sol gravity reading
```

```oneil
# mars_like.one
design galaxy

# Override earth's gravity through the local alias.
g.earth = 3.72 :m/s^2
```

```sh
oneil eval galaxy.on --design mars_like.one
```

```oneil-eval-output
W = 2976 : N  # Landing weight
g_sol = 3.72 : m/s^2  # Sol gravity reading
```

Because `galaxy.earth` and `galaxy.sol.earth` are the same instance, both
`W` (which reads `g.earth` directly) and `g_sol` (which reads through `sol`)
see the updated value.
