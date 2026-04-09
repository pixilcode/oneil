# Parameters

Parameters are the main way to define values in an Oneil model. Parameters are variables with extra metadata for system modeling and review. They define a long name, limits, a math symbol for rendering and review, units, a derivation, and either a value assignment or an equation relating this parameter to others. This section covers the basics: defining parameters, running a model, and selecting what gets printed.

## Hello world

A minimal Oneil model is a single parameter. Create a file `hello.on` with:

```oneil
Hello: x = 1
```

Run it with:

```sh
oneil eval hello.on --params x
```

The `--params x` option prints the parameter `x`. This option can also be shortened to `-p x`. You should see something like:

```text
x = 1  # Hello
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
Total length: total_length = 1
```

Here `Total length` is the label, `total_length` is the name, and `1` is the value.

## Comments

Comments in Oneil are prefixed by `#` and go until the end of the line.

```oneil
# this is a comment
My param: p = 10
My other param: o = 5  # comments don't have to start at the beginning of a line
```

## Running a model and viewing output

To evaluate a model file:

```sh
oneil eval <model>.on
```

For example, with a file `hello.on` containing `Hello: x = 1`:

```sh
oneil eval hello.on
```

Output:

```text
(No performance parameters found)
```

Note that there are no parameters printed out. The reason for this is discussed in [_Annotations_](#annotations).
For now, use `--print all` to print out all parameters.

```sh
oneil eval hello.on --print all
```

Output:

```text
x = 1  # Hello
```

## Multiple parameters and references

You can define multiple parameters in one file. They can reference each other by name, and the order of declarations does not matter - Oneil resolves dependencies automatically.

Example:

```oneil
First: a = 1
Second: b = a + c
Third: c = 2
```

Save as `multi.on` and run:

```sh
oneil eval multi.on --print all
```

Output:

```text
a = 1  # First
c = 2  # Third
b = 3  # Second
```

## Selecting parameters to print

By default, only parameters with certain [annotations](#annotations) are printed. To print specific parameters by name, use `--params` (or `-p`):

```sh
oneil eval <model>.on --params param1,param2
# or
oneil eval <model>.on -p param1,param2
```

Example with `multi.on`:

```sh
oneil eval multi.on -p b,a
```

Output:

```text
b = 3  # Second
a = 1  # First
```

The order in the comma-separated list is the order they appear in the output. You can select one or more parameters; only those are printed.

## Annotations

Parameters can be marked with optional annotations that control whether they are printed by default and how they are used:

| Annotation  | Symbol | Meaning                                                                                                              |
|-------------|--------|----------------------------------------------------------------------------------------------------------------------|
| Trace       | `*`    | Included when printing “trace” parameters (default).                                                                 |
| Debug       | `**`   | Same as trace, and with `--debug` / `-D`, extra debug info is printed for variables used to evaluate this parameter. |
| Performance | `$`    | Marked as a performance variable.                                                                                    |

Annotations appear before the label. Example:

```oneil
# Trace marker:
* First: a = 1

# Debug marker:
** Second: b = a + c

# Performance marker:
$ Third: c = 2
```

With the default print mode (perf), only the performance variables are displayed.

```text
c = 2  # Third
```

To display the other variables, use the `--print`/`-P` argument with the `trace` argument.

```sh
oneil eval <model>.on --print trace
```

```text
a = 1  # First
c = 2  # Third
b = 3  # Second
```

## Evaluating expressions

While it is usually best to include all equations in the model itself, there may
be some times when you need to do a quick evaluation of an expression. For that,
Oneil provides the `--expr`/`-x` argument. This argument prints out the result
of evaluating the provided expression.

For example, assuming the previous model is in `my_model.on`, you can run the
following:

```bash
oneil eval my_model.on --expr "1 + 1"
```

```text
1 + 1 = 2
```

It's great that you can evaluate simple expressions, but the real power of
`--expr` comes when you refer to variables in the model.

```bash
oneil eval my_model.on --expr "b - a"
```

```text
b - a = 2
```

In addition, you can provide multiple expressions at the same time.

```bash
oneil eval my_model.on --expr "b - a" --expr "b - c"
# or, using the shorter name
oneil eval my_model.on -x "b - a" --expr "b - c"
```

```text
b - a = 2
b - c = 1
```
