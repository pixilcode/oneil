# Units

One of Oneil's defining features is its unit-based type system. In other words,
Oneil tracks units and disallows invalid operations, such as adding kilograms
and time or comparing meters and radians. It also provides automatic conversion
for units that have different magnitudes, such as kilometers and meters.

To assign a unit to a parameter, use the `: <unit>` syntax:

```oneil
Distance: d = 100 : meters
Travel time: t = 20 : seconds
```

This will assign `d` to a value of `100 : meters` and `t` to a value of `20 :
seconds`. These values are now _measured numbers_, or numbers with units.

> ![NOTE]
> There are often multiple synonyms for a given unit. For example, the above
> model could also be written as
>
> ```oneil
> Distance: d = 100 : m
> Travel time: t = 20 : s
> ```
>
> To see a list of all builtin units and their synonyms, run
> `oneil builtins unit`. Also, if you would like to search for a given unit, run
> `oneil builtins unit <unit>`.

We can then define velocity as

```oneil
Velocity: v = d/t : m/s
```

defining a parameter with a measured value with the units `m/s`.

Printing out `v` with `oneil eval calc.on --params v` produces

```text
v: v = 5.0000 : m/s  # Velocity
```

If we wanted to, we could just as easily define velocity in kilometers per hour

```oneil
Velocity: v = d/t : km/hr
```

which would produce

```text
v: v = 18.0000 : km/hr  # Velocity
```

Note that we did not have to do any conversions. Oneil handles that for us.
However, if we try to use incorrect units, Oneil will produce an error.

```oneil
Velocity: v = d/t : kg/hr
#                   ^^ `kg` instead of `km`
```

```text
error: parameter value unit `meters/seconds` does not match expected unit `kg/hr`
 --> /tmp/test.on:6:15
  | 
6 | Velocity: v = d/t : kg/hr
  |               ^--
```

In addition, Oneil requires units on _any parameters with measured values_. If
we leave out the unit, we get an error. (For the exception to this, see the
later section on [dimensionless values](#dimensionless-units))

```oneil
Velocity: v = d/t
#                 ^ No unit
```

```text
error: parameter is missing a unit
 --> /tmp/test.on:6:15
  | 
6 | Velocity: v = d/t 
  |               ^--
  = note: parameter value has unit `meters/seconds`
  = help: add a unit annotation `: meters/seconds` to the parameter
```

## Units, dimensions, and magnitude

Before we get more into how units work in Oneil, we're going to take a quick
detour to delve into _what_ makes units compatible. Why can you add meters and
kilometers, but not meters and kilograms? Why is a Joule equivalent to a
Watt-second?

The answer is _dimensions_. A dimension can be defined as _an aspect of
something that can be measured_. That definition is hard to understand on its
own, though, so lets consider the dimension of _time_ as an example.

### A trip to the store (and more)

When measuring how long a car takes to get from your house to the store, it
doesn't matter whether you measure in seconds, minutes, or even millennia. They
all are measurements of the dimension of _time_.

You can also measure how long it takes for you to get from the store to work,
and you can measure that in any unit of _time_ as well. Then, you could add
the values together because they both measure the dimension of _time_.

However, it wouldn't make sense to add the mass of your car to the measured travel time,
since travel time is measured in the dimension of _time_, while car mass is measured in
the dimension of _mass_.

### Supported dimensions

Oneil supports the following dimensions, with their associated base unit.

- mass: `kilogram`
- distance: `meter`
- time: `second`
- temperature: `Kelvin`
- current: `ampere`
- information: `bit`
- currency: `$` (USD)
- substance: `mole`
- luminous intensity: `candela`

Each unit has 0 or more dimensions associated with it. The kilometer is defined
as having a dimension of `distance`, while a `Joule` would have the dimensions
of `mass`, `distance^2`, and `time^-2`.

There are also dimensionless units such `%`. These are discussed
[later in this chapter](#dimensionless-units).

If you haven't quite wrapped your head around dimensions yet, don't worry. You
don't need to fully understand it to use Oneil.

<!-- PONDER: this isn't essential for them to know, it's just helpful for them
             to have reference when we use "dimensionless" rather than "unitless"
             later. Although maybe we just use "unitless" later and move this
             to an "advanced" section of the guide? -->

## Composing units in a unit expression

A **unit expression** is built from **terms** separated by `*` or `/`, which
group **left-to-right** in the usual way for multiplication and division
(left-associative).

Each **term** is one of:

- A **unit name**, optionally raised to a numeric power with `^`, for example `m`, `s^2`, `m^0.5`.
- The literal **`1`**, meaning a dimensionless factor of one. This is common in rates such as `1/s` (per second).
- A **parenthesized** unit expression, for example `J/(kg*K)`, when you need to override the default left-associative grouping.

A unit can be a base unit (such as `km` or `grams`), but base units can
also be combined with other operators.

## Arithmetic and comparison operators

Arithmetic and comparison operator rules and behavior is defined by the
following table. The unit of a given value `x` is indicated by `x_unit`.

| Operation | Input Rules | Unit Output | Example |
| `a + b`, `a - b`, `a % b` | `a_unit` and `b_unit` must have the same dimensions | `a_unit` |
| `a * b` | None | `a_unit * b_unit`; unitless values have a unit of `1` |
| `a / b` | None | `a_unit / b_unit`; unitless values have a unit of `1` |
| `a ^ b` | `b` cannot have a unit | `a_unit ^ b` |
| comparison (`<`, `>`, `<=`, `>=`, `==`, `!=`) | `a_unit` and `b_unit` must have the same dimensions | N/A (produces `true` or `false`) |

### Examples

```oneil
# Addition, subctraction, modulo
test: (1000:m) + (1:km) == (2000:m)
test: (1:km) + (1000:m) == (2:km)
test: (5:min) - (30:s) == (4.5:min)
test: (80:s) % (1:min) == (20:s)

# Multiplication
test: (1:m) * (1:s) == (1:m*s)
test: (1:m) * (1:m) == (1:m^2)
test: (1:m) * 1 == (1:m)

# Division
test: (1:m) / (1:s) == (1:m/s)
test: (1:m) / 1 == (1:m)

# Exponentiation
test: (1:m)^2 == (1:m^2)

# Comparison
test: (1:kg) < (2000:g)
test: (1:kg) > (1:g)
test: (1:kg) <= (1000:g)
test: (1:kg) >= (900:g)
test: (1:kg) == (1000:g)
test: (1:kg) != (1:g)
```

## Unit casting

Imagine that you have a test that takes time to start up before it runs. The full time
of the test is 5 minutes and start-up time is 10 seconds. To calculate what the actual
run time of the test is, you might write the following model.

```oneil
Full time: t_full = 5 : min
$ Run time: t_run = t_full - 10 : min
```

However, this will produce an error:

<!-- TODO: make this error more user friendly -->

```text
error: expected scalar with unit `min` but found scalar
 --> /tmp/test.on:4:23
  | 
4 | Run time: t_r = t_f - 10 : min
  |                       ^-
```

In other words, Oneil can't determine whether `10` is supposed to be 10 seconds,
10 minutes, or 10 hours.

The first recommended solution is to create another parameter to hold this "magic
number". You can then define a unit on that parameter.

```oneil
Full time: t_full = 5 : min
Startup time: t_start = 10 : s
$ Run time: t_run = t_full - t_start : min
```

However, there are some less-contrived situations where you may just want to
label a unitless number with a unit.

To do so, you can use _unit casting_. Unit casting takes the form of
`(<expression> : <unit>)`. This allows a unitless value to be assigned a unit.

Using this, the model could be rewritten as

```oneil
Full time: t_full = 5 : min
$ Run time: t_run = t_full - (10 : s) : min
```

### Verifying units

The most common use case for unit casting is to cast a unitless value into a
measured value. However, unit casting can also be used to verify that a unit
matches the expected unit. For example, you might be trying to debug the
following model, which has a unit error.

```oneil
Start velocity: v_start = 10 : m
#                             ^ should be m/s
End velocity: v_end = 20 : m/s
Time: t = 5 :s
$ Acceleration: a = (v_end - v_start) / t : m/s^2
```

To verify that you are getting the expected units, you could wrap `v_end` and
`v_start` in unit casts to find out where the error is.

```oneil
Acceleration: a = ((v_end : m/s) - (v_start : m/s)) / t : m/s^2
```

This would then reveal that `v_start` is in `m` rather than `m/s`, allowing you
to fix the error, then remove the casts.

This situation is obviously contrived, but using unit casting in this way may
come in handy with more complex operations.

## `strip`

In the case that you would like to treat a measured value as unitless, Oneil
provides the `strip` function. The strip function removes any units from a
value.

```oneil
ADC bit resolution: S_adc = 10 :b
$ ADC step count: n_adc = 2^(strip(S_adc)-1)
```

The places where this should be used are rare and should be treated cautiously
since `strip` effectively disables unit checking.

In addition, it is important to realize that `strip` strips the unit that is
_currently associated with a value_.

```oneil
X in meters: x_m = 1000 : m
X in kilometers: x_km = 1 : km

# 1000 meters == 1 km
test: x_m == x_km

# 1000 != 1
test: strip(x_m) != strip(x_km)
```

For this reason, when using `strip`, it is recommended that you first cast
the value to the unit that you expect it to be.

```oneil
# convert both values to meters before comparing them
test: strip((x_m : m)) == strip((x_km : m))
## Non-linear units

On top of linear units, Oneil supports _decibel_ (**dB**) units. You form a
decibel unit by prefixing `dB` directly to a built-in unit name, for example
`dBmW` (decibels relative to one milliwatt) or `dBV`. The bare name `dB`
(with no following unit) is also valid; it behaves as a dimensionless logarithmic
unit.

Support for other non-linear units is on the roadmap.

When any unit is specified with prefix `dB`, Oneil internally converts the
parameter to the corresponding linear value, performs all calculations in linear
terms, and reconverts the value to `dB` for display. This means that equations
that contain parameters with `dB` units should use linear math. For example,
when calculating the signal to noise ratio by hand, you might subtract the noise
(`dB`) from the signal (`dB`), but in Oneil, you divide the signal by the noise:

``` { .on }
Noise power: P_n = -100 :dBmW
Signal power: P_s = -90 :dBmW
Signal-to-noise ratio: S_N = P_s/P_n
```

## Dimensionless units

There are some units that don't have any dimensions, such as `%` or `ppm` (parts
per million). These units are referred to as _dimensionless units_, and values
with dimensionless units are referred to as _dimensionless values_.

### Unitless equivalence

Dimensionless values can be treated as if they have no unit. The following
demonstrates this with the `%` unit.

```bash
# `100%` is treated as equal to `1`
oneil eval my_model.on \
  -x "(100:%) == 1" \
```

```text
(100:%) == 1 = true
```

```bash
# the `1` is equal to `100%`, not `1%`
oneil eval my_model.on \
  -x "(100:%) + 1"
```

```text
(100:%) + 1 = 200 :%
```

### Angular Units

The lack of distinction between dimensionless values and unitless values is
especially important when it comes to units involving _radians_. The
International System of Units treats radians as dimensionless, and Oneil has
opted to follow this convention. Therefore, all angular units (such as
`radians`, `degrees`, and `revolutions`) are specified in radians. Therefore,
when adding a unitless number to an angular value, the unitless number is
treated as if it is specified in `radians`.

```oneil
test: (1:rad) == 1

test: (1:rad) + 1 == (2:rad)

test: (360:deg) == 2*pi

test: (360:deg) + 2*pi == (720:deg)
```

#### Using `rad/s` and `Hz`

In terms of SI units, there is a problem with `rad/s` and `Hz`. It is that
[`rad/s != Hz` even though `rad/s == 1/s` and `Hz == 1/s`](https://iopscience.iop.org/article/10.1088/1681-7575/ac0240).
In order to get around this problem, Oneil specifies that `1 Hz == 2*pi rad/s`.
This solution helps us to avoid this inconsistency, but it can also lead to
some problems of its own.

```oneil
Frequency: f = 1 : Hz
Cycles in 2 seconds: cycles = f * (2:s) : 1
# we would expect `2`, since 1 Hz = 1 cycle/s
# however, we get `12.57` instead
```

The `cycle` unit is defined as `2*pi radians`, so if you simply want to view the
unit in cycles, add the `cycle` unit.

```oneil
Cycles in 2 seconds: cycles = f * (2:s) : cycles
# prints out `2 cycles`
```

However, using the `cycle` unit on an intermediate value will cause `cycle` units
to propagate throughout the model. In order to avoid this, divide the value in
`Hertz` by `2*pi` to go from radians to cycles.

```oneil
Frequency: f = 5 : GHz
Speed of light: c = 299792458 : m/s

Wavelength: lambda_cycles = c/f : cm/cycle
# convert from Hz in rad/s to Hz in cycle/s
Wavelength: lambda_convert = c/(f/2*pi) : cm
# or, simplified
Wavelength: lambda_convert2 = 2*pi*c/f : cm
```

In the future, we may add a rotation dimension, but for now, this is what is
required to get around the inconsistency.

### Display

As mentioned previously, dimensionless units are mainly kept around so that
values can be displayed correctly to the user. For example, if you define a
parameter in `%`, you often would like the value to be displayed in `%`.

```oneil
$ Average battery charge: charge = 60 : %
# prints out as `60%`
```

However, there are some situations, such as ratios, where you would like the
value to be printed out as a unitless value. Oneil can often detect this and
will print it as such.

```oneil
Length 1: l1 = 10 : m
Length 2: l2 = 20 : m

$ Length ratio: lr = l1 / l2
# prints out as `0.5`
```

There are some cases, though, where Oneil is unable to detect that a value is
intended to be unitless.

```oneil
Length 1: l1 = 0.010 : km
Length 2: l2 = 20 : m

$ Length ratio: lr = l1 / l2
# prints out as `5e-4 km/m`
```

In this case, you can add a unit annotation of `1` to tell Oneil that the value
should be unitless.

```oneil
Length 1: l1 = 0.010 : km
Length 2: l2 = 20 : m

$ Length ratio: lr = l1 / l2 : 1
# prints out as `0.5`
```

You can do a similar thing with percentage units.

```oneil
Probability of A: P_A = 20 : %
Probability of B: P_B = 60 : %

$ Probability of A and B: P_AB = P_A * P_B
# prints out as `1200 %*%`

$ Probability of A and B with unit: P_ABu = P_A * P_B : %
# prints out as `12%`
```

Note that **doing this does not change the value itself**. It only changes
**how the value is printed**. If you do not care how the value is printed (for
example, if it is an intermediate variable), then you do not need to add a unit
to the parameter.

```oneil
Length 1: l1 = 0.010 : km
Length 2: l2 = 20 : m

$ Length ratio 1: lr_1 = l1 / l2
# prints out as `5e-4 km/m`

$ Length ratio 2: lr_2 = l1 / l2 : 1
# prints out as `0.5`

test: lr_1 == lr_2
```
