# Notes

Oneil renders parameter equations for review directly from code. This makes it easier to review code with complex equations.

If you've ever written a scientific paper, you know that there is often a lot of typeset math and narrative involved in deriving an equation. Showing your work like this helps you and others remember or review the reasons a parameter is expressed the way it is. To help you do this, Oneil supports inline LaTeX, called "notes". It's like a built-in documentation system.

## Parameter Notes

To add a note to a parameter, start the following line with the `~` character.

```oneil
Rotation rate: omega = 1 :deg/min

Cylinder radius: r = d/2 :km

    The distance from the center of the cylinder to the inner rim.
```

You can use three tildes to start and end a multi-line note:

```oneil
    Artificial gravity: g_a = r*omega**2 :m/s^2

        ~~~
        The position of a point on the rim of a rotating cylinder is:

        $\vec{r}(t) = r\cos(\omega t)\,\hat{i} + r\sin(\omega t)\,\hat{j}$

        Taking the first derivative gives the velocity:

        $\vec{v}(t) = \frac{d\vec{r}}{dt} = -r\omega\sin(\omega t)\,\hat{i} + r\omega\cos(\omega t)\,\hat{j}$

        Taking the second derivative gives the acceleration:

        $\vec{a}(t) = \frac{d\vec{v}}{dt} = -r\omega^2\cos(\omega t)\,\hat{i} - r\omega^2\sin(\omega t)\,\hat{j} = -\omega^2\vec{r}(t)$

        The acceleration points radially inward (toward the center), and its magnitude is:

        $|\vec{a}| = r\omega^2$

        This centripetal acceleration acts as artificial gravity for inhabitants
        standing on the inner rim of the cylinder, so $g_a = r\omega^2$.
        ~~~
```

## Sections and Section Notes

The `section` keyword will produce a header when rendered. Sections can be given their own notes:

```oneil

    Earth gravity: g_E = 9.81 :m/s^2

    section Tests

        ~ The following tests ensure that the artificial gravity of the station won't exceed a \href{https://www.reddit.com/r/scifiwriting/comments/szwvep/what_is_the_highest_gravity_that_humans_could/}{livable range for human occupants}.

    test : g_a < 1.1*g_E
```
