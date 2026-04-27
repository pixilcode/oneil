# Importing models

One of the purposes of Oneil's models is to represent **collections of systems
and subsystems**. To this end, Oneil provides two different ways to import a
model: as a **reference** or as a **submodel**.

## References

A **reference** makes another model's parameters available under an alias.
References are useful for shared, globally constant data — physical constants, material
properties, standard environments — that many models need to read but that are
not themselves design or design parameters of the current system.

```oneil
# constants.on
Speed of light: c = 299792458 :m/s
Planck constant: h = 6.626e-34 :J*s
Boltzmann constant: k_B = 1.38e-23 :J/K
```

```oneil
# photon.on
reference constants as phys

Photon frequency: f = 5.09e14 :Hz
Photon energy: E = h.phys * f :J
```

```oneil
# link_budget.on
reference constants as phys

Distance: d = 384400 :km
Signal frequency: f = 2.4e9 :Hz
Path loss: L_fs = (4 * pi * d * f / c.phys)^2
```

Both `photon.on` and `link_budget.on` read from the same `constants.on` file.
If a [design](./10-designs.md) overrides a value on `constants`, both models see
the change — references share a single evaluated instance.

The alias after `as` is optional; without it, parameters are accessed using the
model filename as the alias:

```oneil
# without alias — use the filename
reference constants
Photon energy: E = h.constants * f :J
```

## Submodels

A **submodel** also imports a model's parameters, but additionally declares that
the imported model is a *submodel* of the current system. Each `submodel`
statement creates an independent instance of that model, so the same model can
be imported twice under different aliases and each instance behaves separately.

Planets are a natural fit: a spacecraft mission may visit two planets in the same
analysis, and each planet should have its own independent parameters.

```oneil
# planet.on
Surface gravity: g = 9.81 :m/s^2
Radius: R = 6371 :km
```

The planet model uses earth's gravity for a sensible default, but these parameters can be changed by a design.

```oneil
# mission.on
submodel planet as earth
submodel planet as mars
apply mars_design to mars

Spacecraft mass: m = 500 :kg

Weight on Earth: W_e = m * g.earth :N
Weight on Mars:  W_m = m * g.mars  :N
```

> [!NOTE]
> The two `submodel planet` lines each create their own planet instance.
> Changing a parameter on `earth` (via a [design](./10-designs.md)) does
> not affect `mars` which is important for actually designing the planet parameters for Mars.

If no alias is given, the model filename is used:

```oneil
submodel planet
Surface gravity seen: g_local = g.planet :m/s^2
```

## Accessing submodel parameters

Parameters inside a reference or submodel are accessed with
`parameter_id.alias` — the **parameter comes first**, the model second.

This is the reverse of the `object.property` convention in most programming languages.
This choice is more amenable to system engineering: the equations are oriented around the parameters with the model being like a subscript or superscript in the equation, making it easy to read `gravity.mars / gravity.earth` for example.

```oneil
# satellite.on
reference constants as phys
submodel planet as target

Orbital speed: v = sqrt(G.phys * M.target / R.target) :m/s
```

A submodel is also *exported* as part of the current model's structure, so a
parent of `satellite.on` can reach nested parameters via the chain
`param.satellite`, `param.target`, etc.

## Referring to a nested submodel

When a submodel itself contains submodels, you can declare a **local alias**
for one of those inner submodels using `[alias]` at the end of the `submodel`
line. The alias gives you a handle to a deeply-nested component within the current file.

```oneil
# planet.on
Surface gravity: g = 9.81 :m/s^2
Radius: R = 6371 :km
Mass: M = 5.97e24 :kg
```

```oneil
# solar_system.on
submodel planet as earth
submodel planet as mars

Star mass: M_star = 1.989e30 :kg
Earth orbital period: T_earth = 365.25 :days
```

```oneil
# galaxy.on
# Import the solar system and declare `earth` as a local alias here,
# so it can be read and overlaid directly at the galaxy level.
submodel solar_system as sol [earth]

Probe mass: m_probe = 800 :kg

# earth is a local alias here — access its parameters directly
Landing weight: W = m_probe * g.earth :N

# solar-system parameters still go through sol
Star mass: M_star = M_star.sol :kg
Earth orbit: T = T_earth.sol :days
```

The `[earth]` block names aliases from inside `solar_system.on` to declare as
local aliases at the `galaxy` level. The local alias and the one inside `sol`
refer to the **same instance** — a [design](./10-designs.md) applied to `earth`
here also affects any parameter in `sol` that reads from `earth`.

Multiple aliases are separated by commas, and any alias can be renamed locally:

```oneil
# declare local aliases for both planets; rename mars to target
submodel solar_system as sol [earth, mars as target]
```
