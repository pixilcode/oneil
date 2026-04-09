# Value Types

Parameter values can include literals, equations, and imported functions. The main literal types are **numbers**, **strings**, and **booleans**.

## Numbers

Numbers use familiar decimal notation: whole numbers, values with a decimal point, and **scientific notation** (`e` or `E`) when a value is very large or very small. **`inf`** can be used for infinity.

For example,

```oneil
Integer: n = 42
Decimal: d = 3.14
Scientific: e_val = 1.5e3
Infinity: inf_val = inf
```

### Number Operators

**Arithmetic** (for ordinary scalar values):

- `^` - exponentiation
- `*` - multiplication
- `/` - division
- `%` - modulo
- `+` - addition
- `-` - subtraction

**Comparisons** (produce booleans; can be **chained**, e.g. `1 < 2 < 3`):

- `<` - less than
- `>` - greater than
- `<=` - less than or equal
- `>=` - greater than or equal
- `==` - equal
- `!=` - not equal

### Number Examples

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

Strings behave like **fixed strings**, not like growable text in many other languages. Typical uses include modes or categories. For example, a battery's array configuration could be either `'series'` or `'parallel'`. As another example, a remote sensing resolution mode might be `'polar'`, `'track'`, or `'footprint'`.

A string is written in **single quotes** `'...'`. Double quotes are not used for strings.

### String Operators

There is no concatenation or mutation. Strings can only be compared for equality:

- `==` — equal
- `!=` — not equal

### String Examples

```oneil
Mode: mode = 'track'
Mode is track: is_track = mode == 'track'
Mode is not polar: is_polar = mode != 'polar'
```

String values work well with **discrete limits** and **piecewise** parameters; those are covered later in the guide.

## Booleans

Boolean literals are the keywords **`true`** and **`false`**.

### Boolean Operators

- `not` - logical NOT (unary)
- `and` - logical AND
- `or` - logical OR

### Boolean Examples

```oneil
True value: t = true
False value: f = false
True and not false: t_and_not_f = t and not f
True or false: t_or_f = t or f
```

Comparisons on numbers (and string equality) also yield booleans. Booleans are used heavily in **piecewise** parameter conditions and in **`test`** declarations; those topics appear in later chapters.
