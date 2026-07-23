# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.16.0] - 2026-07-23

0.16.0 is the initial release of the Rust rewrite to the public. It has feature
parity with the Python version aside from missing design files and a REPL. There
are also some changes to the syntax and semantics.

### Added

- Unit casting (`(<expr>:<unit>)`) can be used to assign units to literals
  - Example: `(1:kg)`
- LSP support and VS Code Extension

### Changed

- `|` is now an expression operator and can be nested in expressions (ex.
  `(0 | 100) + 273.15`)
- **Breaking:** Strings may now only use single quotes (`'`)
- **Breaking:** Notes are no longer defined by indentation. Instead, use
  `~ my note` for single-line notes and `~~~` to surround multi-line notes.
- **Breaking:** Certain binary operators require the same units on both sides,
  but in the new Oneil, literal numbers are considered to be unitless. This
  means for example that if `x` is in `km`, then `0 | x` or `x > 100` will
  produce a unit mismatch error. Instead, you will need to use unit casting,
  like `(0:km) | x` or `x > (100:km)`.
- **Breaking:** Python API has been completely revamped. See the docs for
  details.
- **Breaking:** Interval arithmetic has been updated to handle more edge cases
- **Breaking:** Units of rotation such as `rad` now have a "rotation" dimension.

### Removed

- **Breaking:** Parameters that use a "pointer" (`=>`) are now obsolete and can
  be replaced with regular parameters.
- **Breaking:** Escaped subtraction (`--`) and escaped division (`//`) have been
  removed. These can be replicated by using the builtin `min`/`max` functions
  and standard subtraction and division operators.

[Unreleased]: https://github.com/careweather/oneil/compare/v0.16.0...HEAD
[0.16.0]: https://github.com/careweather/oneil/releases/tag/v0.16.0
