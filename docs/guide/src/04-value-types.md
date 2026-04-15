# Value Types

Parameter values can include literals, equations, and imported functions. The main literal types are **numbers**, **strings**, and **booleans**.

## Numbers

Numbers use familiar decimal notation: whole numbers, values with a decimal point, and **scientific notation** (`e` or `E`) when a value is very large or very small. **`inf`** can be used for infinity.

For example,

```oneil
100th prime number: p_100 = 541
Golden ratio: phi = 1.618
Avogadro constant: N_A = 6.022e23
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

### Built-in `pi` and `e`

The identifiers **`pi`** and **`e`** are built-in numeric constants (π and Euler’s number). They can be used like any other value in expressions.

## Strings

Strings behave like **fixed strings**, not like growable text in many other languages. Typical uses include modes or categories. For example, a battery's array configuration could be either `'series'` or `'parallel'`. As another example, a remote sensing resolution mode might be `'polar'`, `'track'`, or `'footprint'`.

A string is written in **single quotes** `'...'`. Double quotes are not used for strings.

### String Operators

There is no concatenation or mutation. Strings can only be compared for equality:

- `==` - equal
- `!=` - not equal

### String Examples

```oneil
# battery.on
Battery configuration: config = 'series'
```

```bash
oneil eval battery.on \
  -x "config == 'series'" \
  -x "config == 'array'"
```

```oneil-eval-output
config == 'series' = true
config == 'array' = false
```

## Booleans

Boolean literals are the keywords **`true`** and **`false`**.

### Boolean Operators

- `not` - logical NOT (unary)
- `and` - logical AND
- `or` - logical OR

### Boolean Examples

Comparisons on numbers (and string equality) also yield booleans. Booleans are used in **piecewise** parameter conditions and in **`test`** declarations; those topics appear in later chapters.
