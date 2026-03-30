# Tests

Alongside parameters, Oneil provides tests, which allows users to verify that
certain properties, requirements, and expectations hold.

The syntax for tests is `test: <test-expression>`.

```oneil
test: 1 + 1 == 2

Component A Length: L_A = 5 :cm
Component B Length: L_B = 3 :cm
Max Length: L_max = 10 :cm

test: L_A + L_B <= L_max
```

A test expression can be any expression that produces a boolean (`true` or
`false`). For more information, see [Booleans](04-value-types.md#booleans)
and [Number operations](04-value-types.md#operations) in the next chapter.
