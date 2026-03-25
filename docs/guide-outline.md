# Oneil Guide

This is an outline of the Oneil guide.

Some important details about the guide:

- Each second-level section should be in its own markdown file in the style of
  `##-<section-name>.md`, where `##` is the two digit number of its order and
  `<section-name>` is the name of the section in all lowercase and dashes. Put
  all of this in the `docs/guide` directory.

- Code examples should be used throughout the guide to demonstrate the concepts
  being explained. The code should be wrapped in tickmarks and marked with
  `oneil` as the language. Also, all code inside the tickmarks should be tested
  by creating one or more files in the `/tmp` directory containing the code,
  then running the code with `oneil eval /tmp/<top-model>.on`. If you are ever
  unsure of how to do this, ask how. The code should always run with no errors
  unless the errors are expected. If there are expected errors, make sure to
  include them in the guide.

Here is what to write about each section.

## Installation

Go over how to install Oneil, including:
- downloading a release from Github
- installing from source by running the install script
- installing from source by running `cargo install`

Also mention installing plugin for VS Code or Cursor.

Also include a TODO note that says "Talk about how to install Python library/type
hints"

## Parameters

Do a simple hello world example by simply setting a variable equal to `1`.

Go over the required parts of the parameter, including the label, the name,
and the value. See the grammar and AST for reference.

Note that `oneil eval <model>.on` is the way to run a file. Demonstrate
how to do so and the output thereof.

Show that you can have multiple parameters in one file, that they can reference
each other, and that they don't have to be in any specific order. Only use `+`
if you need an arithmetic operator.

Then show how you can select one or more params to print with
`oneil eval <model>.on --params <param1>,<param2>`. `-p` can be used for short.

### Annotations

Discuss the trace (`*`), debug (`**`), and performance (`$`) annotations. See
the grammar for how they are used with a parameter.

Discuss `--print-mode`/`-m` and how the annotations interact with it. Then
discuss `--debug`/`-D`. See `oneil eval --help` for reference.

### Other Useful Eval Tools

Discuss `--expr`, `--watch`, `--partial`, `--no-header`, `--no-test-report`, and
`--no-parameters`. Also mention that you can use `oneil eval --help` to get
help.


## Value Types

When going over each type, describe it, give an example, and describe operations
that can be performed on it.

### Numbers

Numbers are generally the standard floating point values. See the grammar for
more details on what's accepted. Give different examples.

Also note all of the arithmetic/comparison operations that can be performed. Do
not include escaped subtraction, escaped division, or the `min/max` operator.
Those will be discussed when interval arithmetic is discussed. See the AST for
arithmetic/comparison operators.

Briefly mention builtin `pi` and `e` values here.


### Strings

Strings are not like strings in other languages. Instead, they are intended to
be like C enums. For example, you might use a string to describe which mode
something is in (ex. resolution mode could be 'polar', 'track', or 'footprint').
A string only uses single quotes (`'`), not double quotes (`"`).

They cannot be concatenated or modified, they can only be compared for equality.

These are useful when combined with discrete limits and piecewise parameters,
which will be discussed later.


### Booleans

The two values are `true` and `false`. See the AST for logical operators.

Note that these are mainly used in piecewise piecewise parameters and tests,
which will be covered later.


## Units

A parameter can be assigned a unit

## Intervals

Note interval comparison operations.


## Tests


## Builtin Functions

## Importing Python Functions

## References and Submodels

## Appendix A: Interval Arithmetic

## Appendix B: Builtins Reference

## Appendix C: Python API Reference
