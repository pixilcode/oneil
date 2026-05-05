# Importing models

Models rarely stand alone. When your system is made of parts — subsystems,
environments, shared constants — you'll want to pull those in from other files.
Oneil gives you two ways to do that: **reference** and **submodel**.

## References

A **reference** makes another model's parameters available under an alias. Use
it for shared parameters and models — physical constants, material properties,
standard environments — that belong to the world your system lives in, not to
the system itself.

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
the change — that's what it means to share a reference.

The `as` alias is optional — without it, you access parameters using the model name:

```oneil
# without alias — use the filename
reference constants
Photon energy: E = h.constants * f :J
```

## Submodels

A **submodel** imports a model's parameters and declares it as a part of the
current system. Unlike a reference, each `submodel` statement creates an
independent instance — import the same model twice under different aliases and
each one has its own parameters.

Planets are a natural fit — a mission might visit multiple planets, and each needs its own independent parameters.

```oneil
# planet.on
Surface gravity: g = 9.81 :m/s^2
Radius: R = 6371 :km
```

The planet model defaults to Earth's values — you can override them with a design.

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
> not affect `mars` — so you can configure each planet independently without worrying about interference.

If no alias is given, you access parameters using the model name:

```oneil
submodel planet
Surface gravity seen: g_local = g.planet :m/s^2
```

## Accessing submodel parameters

To access a parameter inside a reference or submodel, write `parameter_id.alias`
— the **parameter comes first**, the model second.

```oneil
# satellite.on
reference constants as phys
submodel planet as target

Orbital speed: v = sqrt(G.phys * M.target / R.target) :m/s
```

A submodel is also *exported* as part of the current model's structure, so a
parent of `satellite.on` can reach nested parameters via the chain
`param.satellite`, `param.target`, etc.

> [!NOTE]
> This is the reverse of the `object.property` convention in most programming
> languages, and it's intentional. In engineering equations, parameters are
> primary — the system or body is a subscript qualifier.

## Referring to a nested submodel

When a submodel itself contains submodels, you can declare a **local alias**
for one of those inner models using `[alias]` at the end of the `submodel`
line — giving you direct access to a nested model.

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

The `[earth]` declaration pulls the `earth` alias from inside `solar_system.on`
up to the `galaxy` level. The local alias and the one inside `sol` are two names
for the **same model** — a [design](./10-designs.md) applied to `earth` here
also affects any parameter in `sol` that reads from `earth`.

You can pull in multiple aliases by separating them with commas, and rename any
of them — useful when the original name is too generic or when you want to
signal intent:

```oneil
submodel solar_system as sol [earth, mars as target]

Probe mass: m_probe = 800 :kg
Weight on Earth: W_earth = m_probe * g.earth :N
Landing weight on target: W_target = m_probe * g.target :N
```