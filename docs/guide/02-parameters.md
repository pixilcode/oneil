# Parameters

Parameters are the main way to define values in an Oneil model. This section covers the basics: defining parameters, running a model, and selecting what gets printed.

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
────────────────────────────────────────────────────────────────────────────────
Model: hello.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
x = 1  # Hello
```

`oneil eval <model>.on` is how you run an Oneil file. The output shows the model path, test summary, and each parameter’s identifier, value, and label (after `#`). In addition, both `oneil e <model>.on` and `oneil <model>.on` can be used as an equivalent to `oneil eval <model>.on`.

## Required parts of a parameter

Each parameter declaration has three required pieces:

1. **Label** - A human-readable name (can include spaces). This can contain any character except the following: `(`, `)`, `[`, `]`, `{`, `}`, `#`, `~`, `:`, `=`, `\n`, `*`, and `$`.

2. **Name** - The identifier used in expressions (e.g. `x`). It must appear after the colon and before `=`. Other parameters and expressions refer to the parameter by this identifier. A name starts with a letter and may contain letters, digits, and underscores (`_`).

3. **Value** - The expression on the right-hand side of `=`. It can be a number, a reference to another parameter, or a more complex expression (with optional unit; see [Value Types](03-value-types.md) and [Units](04-units.md)).

The syntax is:

```oneil
Label: name = value
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
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/hello.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
```

Note that there are no parameters printed out. The reason for this is discussed in [_Annotations_](#annotations).
For now, use `--print all` to print out all parameters.

```sh
oneil eval hello.on --print all
```

Output:

```text
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/hello.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
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
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/multi.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
a = 1  # First
c = 2  # Third
b = 3  # Second
```

## Selecting parameters to print

By default, only parameters with certain annotations are printed. To print specific parameters by name, use `--params` (or `-p`):

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
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/multi.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
b: b = 3  # Second
a: a = 1  # First
```

The order in the comma-separated list is the order they appear in the output. You can select one or more parameters; only those are printed.

## Annotations

Parameters can be marked with optional annotations that control whether they are printed by default and how they are used:

| Annotation  | Symbol | Meaning                                                                                                              |
|-------------|--------|----------------------------------------------------------------------------------------------------------------------|
| Trace       | `*`    | Included when printing “trace” parameters (default).                                                                 |
| Debug       | `**`   | Same as trace, and with `--debug` / `-D`, extra debug info is printed for variables used to evaluate this parameter. |
| Performance | `$`    | Marked for performance/optimization; can be printed in “perf” mode.                                                  |

TODO: mention debug variables and `--debug` seperately

Annotations appear before the label. Example:

```oneil
* Trace param: t = 1
** Debug param: d = 2
$ Perf param: p = 3
```

Save as `annot.on`. With the default print mode (trace), all three are printed because trace mode includes `*`, `**`, and `$`:

```sh
oneil eval /tmp/annot.on
```

Output:

```text
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/annot.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
t = 1  # Trace param
d = 2  # Debug param
p = 3  # Perf param
```

### Print mode and debug

- **`--print` / `-P`** - Controls which parameters are printed when you don’t use `--params`:
  - `trace` (default): print parameters marked with `*`, `**`, or `$`.
  - `perf`: print only parameters marked with `$`.
  - `all`: print every parameter.

  Example: to see only performance-marked parameters:

  ```sh
  oneil eval annot.on -P perf
  ```

  Output:

  ```text
  ...
  p = 3  # Perf param
  ```

## Other useful eval options

- **`--expr` / `-x`** - Evaluate one or more expressions in the model’s context. Example:

  ```sh
  oneil eval multi.on --expr "a + b"
  ```

  Output includes: `a + b = 4`.

- **`--watch`** - Watch the model file for changes and re-run evaluation when it changes.

- **`--debug`** - If evaluation has errors, still print partial results after the error messages.

- **`--no-header`** - Omit the model path and test summary header.

- **`--no-test-report`** - Omit the test report line from the header.

- **`--no-parameters`** - Do not print any parameters (overrides `--params` and `--print`). Useful when you only care about `--expr` output or the test report.

For a full list of options, run:

```sh
oneil eval --help
```

```
