# Parameters

Parameters are the main way to define values in an Oneil model. Parameters are variables with extra metadata for system modeling and review. They define a long name, limits, a math symbol for rendering and review, units, a derivation, and either a value assignment or an equation relating this parameter to others. This section covers the basics: defining parameters, running a model, and selecting what gets printed.

## Hello world

A minimal Oneil model is a single parameter. Create a file `hello.on` with:

```oneil
Hello world: hw = 1
```

Run it with:

```sh
oneil eval hello.on --params hw
```

The `--params hw` option prints the parameter `hw`. This option can also be shortened to `-p hw`. You should see something like:

```text
hw = 1  # Hello world
```

`oneil eval <model>.on` is how you run an Oneil file. The output shows the model path, test summary, and each parameter’s identifier, value, and label (after `#`). In addition, both `oneil e <model>.on` and `oneil <model>.on` can be used as an equivalent to `oneil eval <model>.on`.

## Required parts of a parameter

Each parameter declaration has three required pieces:

1. **Name** - A human-readable name (can include spaces). This can contain any character except the following: `(`, `)`, `[`, `]`, `{`, `}`, `#`, `~`, `:`, `=`, `\n`, `*`, and `$`.

2. **Identifier** - The identifier used in expressions (e.g. `x`). It must appear after the colon and before `=`. Other parameters and expressions refer to the parameter by this identifier. A name starts with a letter and may contain letters, digits, and underscores (`_`).

3. **Expression** - The expression on the right-hand side of `=`. It can be a number, a reference to another parameter, or a more complex expression (with optional unit; see [Value Types](03-value-types.md) and [Units](04-units.md)).

The syntax is:

```oneil
Name: identifier = expression
```

For example,

```oneil
Antenna length: l_a = 10
```

Here `Antenna length` is the label, `l_a` is the name, and `10` is the value.

## Comments

Comments in Oneil are prefixed by `#` and go until the end of the line.

```oneil
# this is a comment
My param: p = 10
My other param: o = 5  # comments don't have to start at the beginning of a line
```

## Running a model and viewing output

To evaluate a model file, use:

```sh
oneil eval <model>.on
```

For example:

```oneil
# antenna.on
Antenna length: a_l = 5
```

```sh
oneil eval antenna.on
```

```text
(No performance parameters found)
```

Note that there are no parameters printed out. The reason for this is discussed in [_Annotations_](#annotations).
For now, use `--print all` to print out all parameters.

```sh
oneil eval antenna.on --print all
```

```text
a_l = 5  # Antenna length
```

> [!NOTE]
> Throughout this guide, we will insert a comment at the top of each model
> indicating the name of the model.
>
> ```oneil
> # my_model.on
> ...
> ```
>
> This way, we can reference it when running `oneil eval`.
>
> ```bash
> oneil eval my_model.on
> ```

## Multiple parameters and references

You can define multiple parameters in one file. They can reference each other by name, and the order of declarations does not matter - Oneil resolves dependencies automatically.

For example,

```oneil
# satellite.on

Body length: l_body = 25
Antenna length: l_a = 5

Satellite length: l_sat = l_body + l_a
```

```sh
oneil eval satellite.on --print all
```

```text
l_body = 25  # Body length
l_a = 5  # Antenna length
l_sat = 30  # Satellite length
```

## Selecting parameters to print

By default, only parameters with certain [annotations](#annotations) are printed. To print specific parameters by name, use `--params` (or `-p`):

```sh
oneil eval <model>.on --params param1,param2
# or
oneil eval <model>.on -p param1,param2
```

For example,

```sh
oneil eval satellite.on -p l_body,l_a
```

```text
l_body = 25  # Body length
l_a = 5  # Antenna length
```

The order in the comma-separated list is the order they appear in the output. You can select one or more parameters; only those are printed.

## Annotations

Parameters can be marked with optional annotations that control whether they are printed by default and how they are used:

| Annotation  | Symbol | Meaning                                                                                                              |
|-------------|--------|----------------------------------------------------------------------------------------------------------------------|
| Trace       | `*`    | Included when printing “trace” parameters (default).                                                                 |
| Debug       | `**`   | Same as trace, and with `--debug` / `-D`, extra debug info is printed for variables used to evaluate this parameter. |
| Performance | `$`    | Marked as a performance variable.                                                                                    |

Annotations appear before the label.

```oneil
# satellite2.on

# No annotation
Mass: m = 10

# Trace annotation:
* Body length: l_body = 25

## Debug annotation:
** Antenna length: l_a = 5

# Performance annotation:
$ Satellite length: l_sat = l_body + l_a
```

```bash
oneil eval satellite2.on
```

```text
l_sat = 30  # Satellite length
```

With the default print mode (perf), only the performance variables are displayed.

To display the other annotated variables, use the `--print`/`-P` argument with the `trace` argument.

```sh
oneil eval <model>.on --print trace
```

```text
l_body = 25  # Body length
l_a = 5  # Antenna length
l_sat = 30  # Satellite length
```

To display _all_ variables, including non-annotated variables, use `--print all`.

```sh
oneil eval <model>.on --print all
```

```text
m = 10  # Mass
l_body = 25  # Body length
l_a = 5  # Antenna length
l_sat = 30  # Satellite length
```

## Evaluating expressions

While it is usually best to include all equations in the model itself, there may
be some times when you need to do a quick evaluation of an expression. For that,
Oneil provides the `--expr`/`-x` argument. This argument prints out the result
of evaluating the provided expression. The expression can include parameters
from the model.

```oneil
# orbit.on
Orbit radius: r = 7000
```

```bash
# determine the orbit circumference
oneil eval orbit.on --expr "2*pi*r"
```

```text
2*pi*r = 43982
```

In addition, you can provide multiple expressions at the same time.

```bash
# determine the orbit circumference and diameter
oneil eval orbit.on --expr "2*pi*r" --expr "2*r"
# or, using the shorter argument name
oneil eval orbit.on -x "2*pi*r" -x "2*r"
```

```text
pi*r^2 = 1.539e8
2*r = 14000
```
