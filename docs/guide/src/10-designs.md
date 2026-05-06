# Designs

A **design file** (extension `.one`) lets you override parameter values, add new
ones, and target specific submodel instances — all without touching the model
itself. Use it to explore "what if" scenarios: alternative materials, different
components, different planetary environments.

<!-- outline:
  1. Design files — syntax and running directly
  2. Applying a design from within a model (apply)
  3. reference vs submodel — instance isolation
  4. Modifying submodel parameters directly (param.ref = value)
  5. Adding performance and other parameters (augmentation)
  6. Submodel aliases\
-->

## Design files

Start your design file with a `design <target>` declaring the model
you're refining. For the parameters themselves, you can use shorthand — just
`identifier = expression` with an optional `:unit`, skipping the `Label: id =
expr` preamble that model files require.

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

If a parameter already exists on the target model, your entry **overrides** it.
If it doesn't exist yet, it's an **addition** — handy for performance parameters
or configuration-specific checks (see [Adding performance and other parameters](#adding-performance-and-other-parameters)).

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

In some models, you want to import a submodel with a design that differs from the default for that submodel. Use `apply <design> to <model_reference>` inside a model to attach a design to
a specific submodel. The design must target the same model that `<ref>` resolves
to — if it doesn't, you'll get an error.

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

You can also apply a design to the *root* model at the command line — no
`apply` line needed in the model file:

```sh
oneil eval planet.on --design mars_gravity.one -P all
```

## `reference` vs `submodel` — design isolation

In some cases you may want to reference a body of parameters that are shared across the model, not limited to a specific component of the system. Whether you use `reference` or `submodel` determines how broadly an applied
design takes effect.

**`reference`** creates a **shared model** — use it for parameters that belong
to the whole system, not to one specific part.

<!-- TODO: This seems like it is a property of the file defining the constants and not the import, though I guess what could be a constant for one person could be a parameter for another. -->

**`submodel`** creates its own independent model. Two `submodel` imports of the
same file are completely independent — a design applied to one does not affect
the other.

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

Since `constants` is a `reference`, if you changed `G` with a design, both
`earth` and `mars` would see the update — which is exactly what you want for
shared environmental data. For the planets themselves, `submodel` isolation
means you can give each one a different design independently.

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

You can override a parameter on a submodel directly from a design using dotted
syntax: `parameter.submodel = value`.

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

This works across multiple levels too. If `mission.on` imports `spacecraft` as
`vehicle`, a mission design can reach into `vehicle`'s thruster directly:

```oneil
# mission.on
submodel spacecraft as vehicle

Mission duration: t_mission = 365 :days
```

```oneil
# boosted_mission.one
design mission

thrust.vehicle.main_engine = 2000 :N
```

If you find yourself overriding several parameters on the same submodel,
consider creating a dedicated design for that submodel and applying it instead
— it keeps each design focused on one model.

If you attempt to assign a value with different units or override a parameter that doesn't exist on the submodel you'll get an
error. To add a new parameter to a submodel, create a design for the submodel and apply it. 

## Adding performance and other parameters

You can also add parameters that don't exist on the target model yet — useful
for performance outputs or configuration-specific calculations that only make
sense for a particular design.

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

New parameters can reference other parameters in the design file as well
as parameters already on the target model, and are evaluated alongside
everything else.

## Submodel aliases

When you declare a local alias for a nested submodel
(described in [Importing models](./09-importing-models.md)), the local alias
and the one inside the intermediate model are two names for the **same model**.
A design applied through either name takes effect on both.

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

Because `galaxy.earth` and `galaxy.sol.earth` are the same model, both reads
pick up the change — `W` going directly through `g.earth`, and `g_sol` going
through `sol`.
