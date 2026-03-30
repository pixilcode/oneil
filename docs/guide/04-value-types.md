# Value Types

Parameter values are expressions. The main literal types are **numbers**, **strings**, and **booleans**.

## Numbers

Numeric values are IEEE-754 double-precision floating-point numbers, written with decimal syntax similar to that of Python. The grammar allows:

- Optional leading sign (`+` or `-`) on a numeric literal.
- Integer and fractional parts (e.g. `42`, `3.14`, `.5`).
- Scientific notation with `e` or `E` (e.g. `1.5e3`, `2E-4`).
- The literal `inf` for infinity.

Examples:

```oneil
Integer: n = 42
Fraction: f = 3.14
Scientific: e_val = 1.5e3
Infinity: inf_val = inf
```

### Operations on numbers

Precedence matches the language grammar (exponentiation before multiplication/division, then addition/subtraction; comparisons and logical operators are lower).

**Arithmetic** (for ordinary scalar values):

| Operator | Meaning |
|----------|---------|
| `-` (prefix) | Unary negation |
| `^` | Exponentiation (right-associative) |
| `*` | Multiplication |
| `/` | Division |
| `%` | Modulo |
| `+` | Addition |
| `-` | Subtraction |

**Comparisons** (produce booleans; can be **chained**, e.g. `1 < 2 < 3`):

| Operator | Meaning |
|----------|---------|
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less than or equal |
| `>=` | Greater than or equal |
| `==` | Equal |
| `!=` | Not equal |

### Example

```oneil
Add: a = 2 + 3
Subtract: s = 10 - 4
Multiply: m = 6 * 7
Divide: d = 20 / 4
Modulo: mo = 17 % 5
Power: p = 2 ^ 8
Comparison: c = 1 < 2
Comparison chain: ch = 1 < 2 < 3
```

### Built-in `pi` and `e`

The identifiers **`pi`** and **`e`** are built-in numeric constants (π and Euler’s number). They can be used like any other value in expressions:

```oneil
Pi: pi_val = pi
Euler's number: euler = e
```

## Strings

Strings behave like **labels**, not like growable text in many other languages. Typical uses include modes or categories. For example, a resolution mode might be `'polar'`, `'track'`, or `'footprint'`.

A string is written in **single quotes** `'...'`. Double quotes are not used for strings.

### Operations

There is no concatenation or mutation. Strings can only be compared for equality:

- `==` — equal
- `!=` — not equal

### Example

```oneil
Mode: mode = 'track'
Mode is track: is_track = mode == 'track'
Mode is not polar: is_polar = mode != 'polar'
```

String values work well with **discrete limits** and **piecewise** parameters; those are covered later in the guide.

## Booleans

Boolean literals are the keywords **`true`** and **`false`**.

### Operations

| Syntax | Meaning |
|--------|---------|
| `not` | Logical NOT (unary) |
| `and` | Logical AND |
| `or` | Logical OR |

`not` binds tightly; `and` binds more tightly than `or`.

### Example

```oneil
True value: t = true
False value: f = false
True and not false: t_and_not_f = t and not f
True or false: t_or_f = t or f
```

Comparisons on numbers (and string equality) also yield booleans. Booleans are used heavily in **piecewise** parameter conditions and in **`test`** declarations; those topics appear in later chapters.
