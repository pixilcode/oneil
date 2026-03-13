# Parameters

Parameters are the main way to define values in an Oneil model. This section covers the basics: defining parameters, running a model, and selecting what gets printed.

## Hello world

A minimal Oneil model is a single parameter. Create a file `hello.on` with:

```oneil
Hello: x = 1
```

Run it with:

```sh
oneil eval hello.on -m all
```

The `-m all` option prints all parameter values (by default only *trace* parameters are printed; see [Annotations](#annotations)). You should see something like:

```
────────────────────────────────────────────────────────────────────────────────
Model: hello.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
x = 1  # Hello
```

So `oneil eval <model>.on` is how you run an Oneil file. The output shows the model path, test summary, and each parameter’s identifier, value, and label (after `#`).

## Required parts of a parameter

Each parameter declaration has three required pieces:

1. **Label** - A human-readable name (can include spaces).

2. **Name** - The identifier used in expressions (e.g. `x`). It must appear after the colon and before `=`. Other parameters and expressions refer to the parameter by this identifier.

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

## Running a model and viewing output

To evaluate a model file:

```sh
oneil eval <model>.on
```

For example, with a file `hello.on` containing `Hello: x = 1`:

```sh
oneil eval hello.on -m all
```

Output:

```
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/hello.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
x = 1  # Hello
```

Without `-m all`, only parameters marked with trace, debug, or performance annotations are printed (see [Annotations](#annotations)).

## Multiple parameters and references

You can define multiple parameters in one file. They can reference each other by name, and the order of declarations does not matter — Oneil resolves dependencies automatically.

Example:

```oneil
First: a = 1
Second: b = a + 2
Third: c = b + a
```

Save as `/tmp/multi.on` and run:

```sh
oneil eval /tmp/multi.on -m all
```

Output:

```
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/multi.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
a = 1  # First
b = 3  # Second
c = 4  # Third
```

`b` uses `a`, and `c` uses both `b` and `a`. Only the `+` operator is used here; other arithmetic and types are covered later.

## Selecting parameters to print

By default, only parameters with certain annotations are printed. To print specific parameters by name, use `--params` (or `-p`):

```sh
oneil eval <model>.on --params param1,param2
# or
oneil eval <model>.on -p param1,param2
```

Example with `/tmp/multi.on`:

```sh
oneil eval /tmp/multi.on -p b,a
```

Output:

```
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

| Annotation | Symbol | Meaning |
|------------|--------|--------|
| Trace     | `*`    | Included when printing “trace” parameters (default). |
| Debug     | `**`   | Same as trace, and with `--debug` / `-D`, extra debug info is printed for variables used to evaluate this parameter. |
| Performance | `$`  | Marked for performance/optimization; can be printed in “perf” mode. |

Annotations appear before the label. Example:

```oneil
* Trace param: t = 1
** Debug param: d = 2
$ Perf param: p = 3
```

Save as `/tmp/annot.on`. With the default print mode (trace), all three are printed because trace mode includes `*`, `**`, and `$`:

```sh
oneil eval /tmp/annot.on
```

Output:

```
────────────────────────────────────────────────────────────────────────────────
Model: /tmp/annot.on
Tests: 0/0 (PASS)
────────────────────────────────────────────────────────────────────────────────
t = 1  # Trace param
d = 2  # Debug param
p = 3  # Perf param
```

### Print mode and debug

- **`--print-mode` / `-m`** — Controls which parameters are printed when you don’t use `--params`:
  - `trace` (default): print parameters marked with `*`, `**`, or `$`.
  - `perf`: print only parameters marked with `$`.
  - `all`: print every parameter.

  Example: to see only performance-marked parameters:

  ```sh
  oneil eval /tmp/annot.on -m perf
  ```

  Output:

  ```
  ...
  p = 3  # Perf param
  ```

- **`--debug` / `-D`** — When set, for parameters marked with `**`, Oneil prints debug information about the variables used to evaluate that parameter. Run `oneil eval --help` for current behavior.

## Other useful eval options

- **`--expr` / `-x`** — Evaluate one or more expressions in the model’s context. Example:

  ```sh
  oneil eval /tmp/multi.on --expr "a + b"
  ```

  Output includes: `a + b = 4`.

- **`--watch`** — Watch the model file for changes and re-run evaluation when it changes.

- **`--partial`** — If evaluation has errors, still print partial results after the error messages.

- **`--no-header`** — Omit the model path and test summary header.

- **`--no-test-report`** — Omit the test report line from the header.

- **`--no-parameters`** — Do not print any parameters (overrides `--params` and `--print-mode`). Useful when you only care about `--expr` output or the test report.

For a full list of options, run:

```sh
oneil eval --help
```
