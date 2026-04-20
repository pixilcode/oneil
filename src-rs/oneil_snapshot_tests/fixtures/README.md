# Test Fixtures

This directory contains fixture files for snapshot tests organized by feature category.
All models use realistic scientific/engineering values for better comprehension.

## Shared Models

Located in the root directory:
- `gravity_earth.on` - Earth surface gravity (g = 9.81 m/s²)
- `gravity_mars.on` - Mars surface gravity (g = 3.72 m/s²)

These planetary constants are shared across multiple test categories for weight/force calculations.

## Test Categories

### `basic/` - Basic Model Tests (3 tests)
- `basic.on` - F = ma calculation (10kg × 9.81m/s² = 98.1N) with test assertion
- `syntax_error.on` - Model with syntax error for error handling
- `failing_test.on` - Model with intentionally failing test (49.05N < 100N)

### `design_overlay/` - Design Overlay Tests (4 tests)
Tests for `use design` and parameter overlay features using planetary gravity.

- `shared_ref.on` + `shared_ref_design.one` - Shared refs with gravity overlay
  - Design targets `gravity_earth` and overrides `g`
  - Parent uses `ref ../gravity_earth as planet` so all reads share one instance
  - Applied `for planet`, the overlay is observed by both `g.planet` reads
- `two_instances.on` + `two_instances_design.one` - Two unique instances with overlay on one
  - Design targets `gravity_earth` and overrides `g`
  - Parent uses `use ../gravity_earth as planet_a`/`planet_b` (distinct instances)
  - Applied `for planet_a`, the overlay only affects `planet_a`
- `nested_param/` - Spacecraft thruster override (uses --design flag pattern)
  - Tests `thrust.main_thruster = 1000 :N` syntax to override child instance params
  - Child thruster: 500N default, overridden to 1000N
  - No wrapper file needed - design applied directly via test harness
- `wrong_target/` - Error case: design's `design <model>` target does not match
  the model the reference resolves to
  - Design targets `gravity_mars` but is applied `for planet` (which is bound
    to `gravity_earth`)
  - Resolver should produce a clear "design target / reference mismatch" error

### `reference_replacement/` - Reference Replacement Tests (5 tests)
Tests for `use model as alias` replacement in design files.

- `ref_replace_simple.one` - Error case: cannot ref and replace same alias in one file
  - Shows error when trying to both create and replace a reference in the same file
- `simple_parent.on` - Landing calculation using Earth gravity
- `simple_replace_design.one` - Replace Earth with Mars gravity
- `simple_replace_eval.on` - Evaluate Mars landing weight
- `with_submodels/` - Propulsion system with thrust-to-weight calculations
  - `mid_with_matching.on` - Earth-based propulsion (T/W ≈ 1.02)
  - `mid_with_different.on` - Mars-based propulsion (T/W ≈ 2.69)
  - Tests replacement of propulsion systems and extracted gravity refs

### `with_clause/` - With Clause Tests (1 test)
Tests for `use model as alias with [submodel]` extraction.

- `child_with_ref.on` - Satellite with Earth gravity reference
- `with_parent_direct.on` - Mission extracting gravity for surface calculations

### `extracted_submodels/` - Extracted Submodel Replacement Tests (1 test)
Tests that extracted submodels resolve through replaced parent references.

- `extract_child.on` - Earth orbital parameters (radius: 149.6M km, period: 365.25 days)
- `extract_child_alt.on` - Mars orbital parameters (radius: 227.9M km, period: 687 days)
- `extract_original.on` - Solar system with Earth orbit
- `extract_replacement.on` - Solar system with Mars orbit
- `extract_parent.on` - Mission planning extracting orbital period
- `extract_design.one` - Replace Earth mission with Mars mission
- `extract_eval.on` - Evaluate mission with Mars orbital period

### `use_design_merge/` - Design Merge Tests (1 test)
Tests for `use design` without `for` (design inheritance/merge).

- `use_design_nofor_parent.on` - Evaluate gravity with design chain
- `use_design_nofor_base.one` - Base design: Moon gravity (1.62 m/s²)
- `use_design_nofor_derived.one` - Derived: custom low-g (5.0 m/s²)

## Test Coverage Summary (15 tests)

| Category | Tests | What's Tested |
|----------|-------|---------------|
| Basic | 3 | Parsing, evaluation, syntax errors, test failures |
| Design Overlay | 4 | Shared refs, unique instances, nested param override, wrong-target error |
| Reference Replacement | 5 | `use model as alias` replacement, with submodels |
| With Clause | 1 | `with [submodel]` extraction base case |
| Extracted Submodels | 1 | Extracted submodels resolve through replaced parent |
| Design Merge | 1 | `use design` without `for` (inheritance) |

## Scientific Constants Used

| Body | Gravity (m/s²) | Orbital Radius (km) | Orbital Period (days) |
|------|----------------|---------------------|----------------------|
| Earth | 9.81 | 149.6×10⁶ | 365.25 |
| Mars | 3.72 | 227.9×10⁶ | 687 |
| Moon | 1.62 | - | - |
