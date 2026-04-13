# Importing models

One of the purposes of Oneil's models is to be able to represent **collections
of systems and subsystems**. To this end, Oneil provides two different ways to
import a model.

The first way to import a model is as a **reference**. When a model is imported
as a reference, all of the *reference model parameters* are made available
through the *reference alias*. The *reference alias* is either the alias
provided or, if there isn't one, the name of the model.

```oneil
# constants.on
Gravity of Earth: g = 9.8 :m/s^2
```

```oneil
# box.on
Mass of box: m_b = 5 :kg

# reference with alias
ref constants as c
Weight of box: w_b = m_b * g.c :N

```oneil
# box2.on
Mass of box: m_b = 5 :kg

# reference without alias
ref constants
Weight of box: w_b = m_b * g.constants :N
```

The second way to import a model is as a **submodel**. Like with a reference,
all of the *submodel parameters* are available through the *submodel alias*. In
addition to this, the model is also exported as a *submodel* of the current
model. This means that the imported model can be referenced as `model.submodel`.

```oneil
# radar.on
Radar cost: cost = 1000 :$
```

```oneil
# solar_panel.on
Solar panel cost: cost = 500 :$
```

```oneil
# satellite.on
use radar
use solar_panel as solar

Satellite cost: cost = cost.radar + cost.solar :$
```

```oneil
# product.on
use satellite
ref satellite.radar
ref satellite.solar_panel as solar
# ... or using `with` syntax ...
use satellite with [radar, solar_panel as solar]
```

Note that in the case of a submodel,
*the submodel and reference name may be different*. If an alias is provided, it
will be used as the reference name, but
not as the submodel name. The submodel name will always be the name of the
model.
