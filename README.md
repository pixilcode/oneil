# The Oneil Design Language

<!-- markdownlint-disable-next-line no-inline-html -->
<img alt="The Oneil Logo" src="docs/icons/oneil-logo.svg" align="right" width="25%">

Oneil is a design specification language for rapid, comprehensive system modeling.

Traditional approaches to system engineering are too cumbersome for non-system engineers who don't have all day. Oneil makes it easy for everyone to contribute to the central source of system knowledge. With Oneil everyone can think like a system engineer and understand how their design impacts the whole.

Oneil enables specification of a system *model*, which is a collection of *parameters*, or attributes of the system. The model can be used to evaluate any corresponding *design* (which is a collection of value assignments for the parameters of the model).

## Features

Oneil makes it easier than ever to build, debug, explore, and version-control models and designs of complex systems.

* Fully-updated design with every modification (no more passing results back and forth)
* Seamless background unit handling (say goodbye to conversions).
* Single source of truth for equations (united documentation and code).
* Automatic calculation of extreme range of possibilities.
* Built-in tests and reality checks.
* Python extensibility.
* VSCode highlighting and linting.

To learn more about how to write syntax, see [the guide](https://pixilcode.github.io/oneil/).

## Examples

A model is a list of parameters. Independent parameters hold values (with optional units); dependent parameters are equations that reference other parameters:

```oneil
# satellite.on

Body mass: m_body = 12 :kg
Antenna mass: m_ant = 0.8 :kg

$ Total mass: m = m_body + m_ant :kg
```

Intervals capture uncertainty or design ranges, and tests check requirements against the evaluated model:

```oneil
# battery.on

Cell voltage: V_cell = 3.6 | 4.2 :V
Cell count: n = 4
Bus voltage limit: V_max = 18 :V

$ Pack voltage: V = n * V_cell :V

test: V <= V_max
    ~ Pack voltage must stay within the bus limit
```

## Roadmap

* [ ] LSP: Go to references
* [ ] LSP: Rename
* [ ] LSP: Document highlighting
* [ ] Typed python imports
  * Allows for type checking to be seperate from evaluation
  * Don't need to carry around unit information during evaluation
* [ ] Separate type checking and evaluation
* [ ] "Designs" redesign
* [ ] "use" -> "submodel"
* [ ] Remove references?
* [ ] Custom dimensions

## Contributing

If you've found a bug or would like to request a feature, feel free to [submit an issue](https://github.com/careweather/oneil/issues)!

If you would like to contribute code, read [`CONTRIBUTING.md`](CONTRIBUTING.md), then feel free to [submit a pull request](https://github.com/careweather/oneil/pulls)!

## About

The initial methodology that inspired Oneil was proposed in Chapter 3 of [Concepts for Rapid-refresh, Global Ocean Surface Wind Measurement Evaluated Using Full-system Parametric Extrema Modeling](https://scholarsarchive.byu.edu/cgi/viewcontent.cgi?article=10166&context=etd), by M. Patrick Walton. For that work, the methodology was painfully implemented in a Google sheet. The conclusion provided ideas and inspiration for early versions of Oneil.

Oneil was developed at Care Weather Technologies, Inc. to support design of the Veery scatterometer. Veery is designed to perform as well as $100M heritage scatterometers at orders of magnitude less cost. This dramatic improvement is facilitated in part by Oneil's streamlined systems engineering capabilities.

Oneil is named after American physicist and space activist [Gerard K. O'Neill](https://en.wikipedia.org/wiki/Gerard_K._O%27Neill) who proposed the gargantuan space settlements known as [O'Neill cylinders](https://en.wikipedia.org/wiki/O%27Neill_cylinder). We built Oneil to meet our own needs, but we hope it stitches together the many domains required to make O'Neill cylinders and move humanity up the [Kardashev scale](https://en.wikipedia.org/wiki/Kardashev_scale).
