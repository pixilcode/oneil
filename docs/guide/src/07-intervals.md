# Intervals

In some cases, you may want to calculate using a range of values rather than one
single value. This allows you to represent uncertainty. For example, maybe you
don't know what the wind speed will be exactly, but you can estimate that it
will be between 5 and 10 kilometers per hour.

To enable this, Oneil provides an _interval operator_ in the form of
`<expr> | <expr>`.

> [!NOTE]
> You may also see references to this as the _minmax operator_, since it
> produces the min and the max values.

The interval operator takes two expressions and produces a value representing
the range from the minimum of the expressions to the maximum expressions. These
values can then be retrieved using the `min` and `max` functions.

```oneil
# temperature.on
$ Ambient temperature: t_amb = 300 | 400 :K
$ Ambient temperature range: t_amb_range = max(t_amb) - min(t_amb) :K
```

```bash
oneil eval temperature.on
```

```oneil-eval-output
t_amb = 300 | 400 :K  # Ambient temperature
t_amb_range = 100 :K  # Ambient temperature range
```

## Arithmetic Operators

The same operators that apply to scalar values apply to interval values: `+`,
`-`, `*`, `/`, `%`, and `^`.

Because an interval is a _range_ of possible values, not a single number,
results can differ from the naive idea of "min with min, max with max." For
example, with subtraction, that naive rule would be wrong:

```oneil
X: x = 10 | 15
Y: y = 0 | 5

Z: z = x - y
#    = (10 | 15) - (0 | 5)
#    = 10 - 0 | 15 - 5
#    = 10 | 10  # incorrect
```

Oneil implements subtraction so the range is arithmetically correct:
`min(i1) - max(i2) | max(i1) - min(i2)`.

```oneil
X: x = 10 | 15
Y: y = 0 | 5

Z: z = x - y
#    = (10 | 15) - (0 | 5)
#    = 10 - 5 | 15 - 0
#    = 5 | 15
```

For more detail on interval operators, see the
[interval arithmetic paper review](../../research/2025-11-13-interval-arithmetic-paper-review.md)
or the implementation in the codebase.

### Escaping and relationships

Oneil’s interval arithmetic aims to satisfy the
[_inclusion property_](../../research/2025-11-13-interval-arithmetic-paper-review.md#inclusion-property):
if every interval in an expression is replaced by some scalar inside that
interval and the expression is evaluated as scalars, the scalar result lies
inside the interval you get by evaluating the expression on intervals.

Bounds can still be _wider_ than necessary. For example, you would expect `a -
a` to be `0` for any `a`. If `a` is `0 | 1`, interval subtraction yields `-1 |
1`. That interval still contains the true result `0`, but it is looser than
`0 | 0`. This know as the
[dependency problem](https://en.wikipedia.org/wiki/Interval_arithmetic#Dependency_problem).

When you need tighter results (for example in geometry, where identities like
`a - a = 0` matter), you can leave “pure” interval arithmetic by using
`min(i)` and `max(i)` to work on scalars, then build a new interval with `|`.
For instance, instead of `a - a`, you can use
`min(a) - min(a) | max(a) - max(a)`.

For common cases, Oneil provides `--` and `//`:

| Operator | Equivalent to                        |
|----------|--------------------------------------|
| `a -- b` | `min(a) - min(b) \| max(a) - max(b)` |
| `a // b` | `min(a) / min(b) \| max(a) / max(b)` |

## Comparison Operators

Intervals can be compared with `==`, `!=`, `<`, `<=`, `>`, and `>=`. The rules
are defined in terms of `min` and `max`:

| Operator | Equivalent to                           | Description                                                           |
|----------|-----------------------------------------|-----------------------------------------------------------------------|
| `a == b` | `min(a) == min(b) and max(a) == max(b)` | The min and the max are the same                                      |
| `a != b` | `min(a) != min(b) or max(a) != max(b)`  | The min or the max is not the same                                    |
| `a < b`  | `max(a) < min(b)`                       | The max value of `a` is less than the min value of `b`                |
| `a <= b` | `max(a) <= min(b)`                      | The max value of `a` is less than or equal to the min value of `b`    |
| `a > b`  | `min(a) > max(b)`                       | The min value of `a` is greater than the max value of `b`             |
| `a >= b` | `min(a) >= max(b)`                      | The min value of `a` is greater than or equal to the max value of `b` |
