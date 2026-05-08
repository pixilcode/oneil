# Migration Guide: Python Oneil → Rust Oneil

This guide covers the syntax and behaviour changes needed to convert model files
written for the Python implementation of Oneil (≤ 0.12.x, as seen in the
`veery` project) to the Rust implementation.

---

## Quick reference

| Topic | Python (old) | Rust (new) |
|---|---|---|
| Version header | `#oneil-0.12.1` | Remove entirely |
| Shared model import | `use constants as c` | `reference constants as c` |
| Independent model import | `use battery as b` | `submodel battery as b` |
| Python function import | `import functions` | `import functions` (unchanged) |
| Piecewise assignment | `x =>{a if cond :unit` | `x = {a if cond :unit` |
| Notes | Indented plain text below param | `~ single line` or `~~~\nmulti-line\n~~~` |
| Section headers | `# # # # # #\nsection Foo\n# # # # # #` | `section Foo` |
| Interval | `3.8\|4.2` | `3.8 \| 4.2` |
| Euler's number | `exp(x)` (Python function) | `e ^ x` (built-in constant `e`) |
| Common math functions | Requires `import functions` | Many are built-in (see below) |

---

## Detailed changes

### 1. Remove version headers

Python files begin with a version comment and optional model description:

```oneil
# Old (Python)
#oneil-0.12.1
  Scatterometer radar characteristics
```

Remove the `#oneil-X.Y.Z` line entirely. A plain comment for the description is
still fine:

```oneil
# New (Rust)
# Scatterometer radar characteristics
```

---

### 2. Imports: `use` → `reference` or `submodel`

The Python `use` keyword is replaced by two distinct keywords in Rust that make
the sharing semantics explicit.

**`reference`** — shared instance. Use for environmental data, constants, and
lookup tables that all consumers should see identically.

```oneil
# Old
use constants as c
use orbit as O
```

```oneil
# New
reference constants as c
reference orbit as O
```

**`submodel`** — independent instance. Use for components and subsystems where
each instance may have different parameter values (especially important when
applying designs).

```oneil
# Old
use battery as b        # if each consumer needs its own independent battery
```

```oneil
# New
submodel battery as b
```

> **Rule of thumb:** if two models `use` the same file and a change to one
> should be visible in both, it was a shared reference — use `reference`.
> If the intent was independent copies, use `submodel`.

---

### 3. Piecewise values: drop the `=>`

The Python piecewise syntax uses `=>` before the first `{…if…}` branch.  The
Rust syntax uses an ordinary `=` followed by the branches.

```oneil
# Old (Python) — note the `=>` and unit attached to first branch
Maximum array current: I_max =>{I_max_c if C_a == 'series' :A
                               {I_max_c*n if C_a == 'parallel'
```

```oneil
# New (Rust) — plain `=`, unit comes after all branches
Maximum array current: I_max = {I_max_c if C_a == 'series'
                                {I_max_c*n if C_a == 'parallel' :A
```

Key differences:
- Replace `=>` with `=`.
- The unit (`:A`, `:V`, etc.) must appear **after the last branch**, not on the
  first branch.

---

### 4. Notes

Python notes are any lines indented below a parameter (tab or spaces) that are
not parameter declarations themselves.  In Rust, notes use an explicit marker.

**Single-line note** — use `~` on the line immediately after the parameter:

```oneil
# Old
Chassis mass: m_chassis = 350 :kg

	Aluminium structure. Sized for 20 g launch loads.
```

```oneil
# New
Chassis mass: m_chassis = 350 :kg

    ~ Aluminium structure. Sized for 20 g launch loads.
```

**Multi-line note** — use `~~~` delimiters.  A `~` note covers exactly one
line; everything after it is parsed as a new declaration.  Any note that spans
more than one line **must** use `~~~`:

```oneil
# Old
Power system mass: m_power = 280 :kg

	Multi-mission radioisotope thermoelectric generator (MMRTG).
	Output: $\approx 110\,\mathrm{W_e}$ at beginning of mission,
	decaying as $P(t) = P_0 e^{-\lambda t}$.
```

```oneil
# New
Power system mass: m_power = 280 :kg

    ~~~
    Multi-mission radioisotope thermoelectric generator (MMRTG).
    Output: $\approx 110\,\mathrm{W_e}$ at beginning of mission,
    decaying as $P(t) = P_0 e^{-\lambda t}$.
    ~~~
```

**Design-file notes** also use `~` or `~~~`, and must appear before the
`design` declaration:

```oneil
# New (.one file)
~ Heavy-duty Rover A: breaks the propellant budget.

design mission_budget

m.rover_a = 8000 :kg
```

Notes support inline LaTeX (`$...$`), display math (`$$...$$` or
`\begin{equation}...\end{equation}`), and `\cite{key}` citations for rendering
in the rendered view.

---

### 5. Section headers

Python often surrounds `section` headers with decorative comment lines.  In
Rust, `section` is a keyword and no decoration is needed.

```oneil
# Old
# # # # # # # # # #
section Battery Cell
# # # # # # # # # #
```

```oneil
# New
section Battery Cell
```

Sections can also have their own notes in Rust:

```oneil
section Power

    ~~~
    Power is the primary mission constraint.
    ~~~
```

---

### 6. Intervals

Both implementations use `|` for intervals, but Rust requires spaces around
the operator:

```oneil
# Old
Average voltage: V_c = 3.8|4.2 :V
Frequency: f = 5.255|5.259 :GHz
```

```oneil
# New
Average voltage: V_c = 3.8 | 4.2 :V
Frequency: f = 5.255 | 5.259 :GHz
```

Rust also adds **interval-safe arithmetic operators** to avoid incorrect
range calculations:
- `--` — interval subtraction (`min - max | max - min`)
- `//` — interval division

---

### 7. Built-in math functions

Many functions that previously required `import functions` (a Python module)
are now built into the Rust evaluator.  You can remove the `import functions`
line if it was only needed for these:

| Function | Status |
|---|---|
| `sqrt(x)` | Built-in |
| `abs(x)` | Built-in |
| `sin(x)`, `cos(x)`, `tan(x)` | Built-in |
| `asin(x)`, `acos(x)`, `atan(x)` | Built-in |
| `log2(x)`, `log10(x)` | Built-in |
| `floor(x)` | Built-in |
| `min(a, b)`, `max(a, b)` | Built-in |
| `exp(x)` | Use `e ^ x` — `e` is a built-in constant |
| `log(x, base)` | Use `log10(x) / log10(base)` |
| `atan2(y, x)` | Still requires Python import |
| `ceil(x)` | Still requires Python import |
| `round(x)` | Still requires Python import |
| Model-specific interpolation (e.g. `theta_3`, `eta_a`) | Still requires Python import |

Keep `import functions` (or a named Python module) for any functions not
listed as built-in above.

---

### 8. Euler's number `e`

`e` is a built-in constant in Rust Oneil (like `pi`).  Replace calls to a
Python `exp` function with the `^` operator:

```oneil
# Old
m_req = m_dry * (exp(dv / v_e) - 1) :kg
```

```oneil
# New
m_req = m_dry * (e ^ (dv / v_e) - 1) :kg
```

---

### 9. Commented-out parameters

Python files sometimes use `#` to comment out alternative parameter values:

```oneil
# Old
Average voltage: V_c = 3.8 | 4.2 :V

#Average voltage: V_c = 4.0 :V
#   Simplified single-point estimate
```

In Rust the `#` comment syntax is the same, so the line is still silently
ignored.  However, for model variants the idiomatic Rust approach is a
**design file** (`.one`):

```oneil
# simplified_voltage.one
design battery

V_c = 4.0 :V
```

This makes the variation explicit, composable, and renderable.

---

### 10. `href` links in notes

Python notes sometimes embed `\href{url}{text}` directly in plain note text.
These render correctly in the Rust rendered view since KaTeX supports `\href`,
but they must now be inside a proper `~` or `~~~` note:

```oneil
# Old (plain-text note, no marker)
Match efficiency: eta_m = 1-abs(Gamma)^2

	\href{https://electronics.stackexchange.com/...}{Source.}
```

```oneil
# New
Match efficiency: eta_m = 1-abs(Gamma)^2

    ~ \href{https://electronics.stackexchange.com/...}{Source.}
```

---

## Checklist for migrating a file

1. [ ] Remove `#oneil-X.Y.Z` version header
2. [ ] Convert `use X as Y` → `reference X as Y` (or `submodel X as Y`)
3. [ ] Remove `import functions` if only used for built-in math (see §7)
4. [ ] Change `=>{...` piecewise → `= {...`, move unit to end of last branch
5. [ ] Wrap all notes in `~` (single line) or `~~~...~~~` (multi-line)
6. [ ] Remove `# # # # # #` section decorations, keep `section Name`
7. [ ] Add spaces around `|` in interval literals
8. [ ] Replace `exp(x)` with `e ^ x`
9. [ ] Move commented-out alternative values to design files where appropriate
