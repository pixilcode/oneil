//! Integration snapshot tests for Oneil evaluation output and errors.
//!
//! Each test runs the full pipeline (parse → resolve → eval) on a fixture
//! and compares the formatted output against a stored snapshot.
//!
//! Tests are grouped by feature category with prefixes (e.g., `basic_`, `overlay_`)
//! so snapshot files sort together.

use std::path::PathBuf;

use crate::util::{run_model_and_format, run_model_and_format_with_design};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// =============================================================================
// Basic Tests
// =============================================================================

#[test]
fn basic_model() {
    let path = fixture_path("basic/basic.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn basic_syntax_error() {
    let path = fixture_path("basic/syntax_error.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn basic_failing_test() {
    let path = fixture_path("basic/failing_test.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

// =============================================================================
// Design Overlay Tests
// =============================================================================

#[test]
fn overlay_shared_ref() {
    // `ref` makes `planet` a shared instance, so an overlay on it is observed
    // by both reads.
    // Expected: w_a = 372 N, w_b = 372 N, total = 744 N (Mars gravity overlay)
    let path = fixture_path("design_overlay/shared_ref.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn overlay_two_instances() {
    // `use` creates unique instances, so an overlay applied `for planet_a`
    // affects only planet_a; planet_b retains the default Earth gravity.
    // Expected: w_a = 372 N (Mars), w_b = 981 N (Earth), total = 1353 N
    let path = fixture_path("design_overlay/two_instances.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn overlay_wrong_target_for_ref() {
    // Error case: applying a design `for r` whose declared target model does
    // not match the model that `r` resolves to should produce a clear
    // resolution error (instead of silently doing nothing).
    let path = fixture_path("design_overlay/wrong_target/parent.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn overlay_nested_parameter() {
    // Tests: param.instance = value syntax (nested parameter override)
    // nested_param_design.one: thrust.main_thruster = 1000 :N
    // Expected: 1000N (from overlay) instead of 500N (default)
    let model = fixture_path("design_overlay/nested_param/nested_param_parent.on");
    let design = fixture_path("design_overlay/nested_param/nested_param_design.one");
    let output =
        run_model_and_format_with_design(&model, Some(&design), Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

// =============================================================================
// Reference Replacement Tests
// =============================================================================

#[test]
fn replace_same_file_error() {
    // Error case: cannot replace a reference in the same file where it's defined
    // The ref and use replacement should be in separate files
    let path = fixture_path("reference_replacement/ref_replace_simple.one");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn replace_via_design_file() {
    // Basic reference replacement via design file
    // simple_parent.on uses design_child (x=10)
    // simple_replace_design.one replaces with design_child_alt (x=99)
    // Expected: s = 99
    let path = fixture_path("reference_replacement/simple_replace_eval.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn replace_mid_direct_baseline() {
    // Baseline: mid_with_matching works directly (x=10)
    let path = fixture_path("reference_replacement/with_submodels/test_mid_direct.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn replace_with_clause_baseline() {
    // Baseline: with [inner] clause works with mid model
    let path = fixture_path("reference_replacement/with_submodels/test_parent_direct.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn replace_with_submodels() {
    // Replacement when parent uses with [inner]
    // Expected: v = 99 (from replaced mid's design_child_alt)
    let path = fixture_path("reference_replacement/with_submodels/replace_submodel_eval.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

// =============================================================================
// With Clause Tests
// =============================================================================

#[test]
fn with_base_case() {
    // `with [inner]` extracts the gravity submodel out of the satellite,
    // making `g.inner` available on the parent.
    // Expected: weight = 100 kg * 9.81 m/s^2 = 981 N
    let path = fixture_path("with_clause/with_parent_direct.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

// =============================================================================
// Extracted Submodel Tests
// =============================================================================

#[test]
fn extract_through_replaced_parent() {
    // Extracted submodels resolve through replaced parent
    // Expected: r = 99 (from replacement's x, not original's 10)
    let path = fixture_path("extracted_submodels/extract_eval.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn with_overlay_propagation() {
    // A `with [inner]` clause aliases the extracted submodel onto the same
    // instance as the deeper child it was lifted from — `parent.inner` and
    // `parent.mid.inner` are two reference-name aliases for one
    // `EvalInstanceKey`. An overlay setting `value.inner = 99` on the parent
    // must therefore land on that single shared instance, so all three reads
    // observe the overridden value:
    //   direct      = value.inner      = 99
    //   via_mid     = value.mid.inner  = 99   (same instance reached via mid)
    //   doubled_mid = doubled.mid      = 198  (mid's own param reads inner)
    let model = fixture_path("with_overlay_propagation/parent.on");
    let design = fixture_path("with_overlay_propagation/overlay.one");
    let output =
        run_model_and_format_with_design(&model, Some(&design), Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

// =============================================================================
// Design Merge Tests
// =============================================================================

#[test]
fn merge_use_design_without_for() {
    // use design file (without for) to merge designs
    let path = fixture_path("use_design_merge/use_design_nofor_parent.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn merge_nested_design_replaces_under_prefix() {
    // When an outer design declares `use design Y for r` and `Y` declares a
    // reference replacement, that replacement targets `r`'s reference (not
    // the outer model's). Here `inner_design` replaces `inner_model.planet`
    // with Mars; pulled in under `r` from `outer_design`, it must reach
    // `out.r.planet` and produce Mars gravity (3.72 m/s^2).
    let path = fixture_path("nested_design_replace/eval.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn merge_propagates_parameter_additions() {
    // When one design inherits another via `use design Y` (no `for`, same
    // target), `Y`'s `parameter_additions` are visible to the consumer's own
    // overlays. Here the consumer overrides `output = derived` where
    // `derived` is added by the imported design, so the override resolves to
    // `2 * base = 10` at eval time.
    let path = fixture_path("design_addition_propagation/eval.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

// =============================================================================
// Design Local Parameter Tests
// =============================================================================

#[test]
fn design_local_augmentation() {
    // Design file adds new parameters that don't exist on the target
    // target.on: radius = 5
    // augment.one: radius = 10 (override), diameter = 2 * radius, circumference = pi * diameter
    // Expected: radius = 10, diameter = 20, circumference = 62.83...
    let model = fixture_path("design_local/target.on");
    let design = fixture_path("design_local/augment.one");
    let output =
        run_model_and_format_with_design(&model, Some(&design), Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn design_augmented_reference_params() {
    // Parent model applies a design to a reference, then accesses augmented params
    // child.on: base = 10
    // augment.one: doubled = 2 * base, constant = 42
    // parent.on: use child as c; use design augment for c; y = c.doubled
    // Expected: x = 10, y = 20, z = 42, total = 72
    let model = fixture_path("augmented_refs/parent.on");
    let output = run_model_and_format(&model, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn overlay_anchor_scope() {
    // Scoped overlay RHS (e.g. `base.a = 2 * multiplier`) must be evaluated in the
    // design's target scope (the anchor), not in the ref's instance scope. `multiplier`
    // lives on parent, so `base.a` resolves to 2 * parent.multiplier = 20 while
    // `base.b` stays 5.
    let model = fixture_path("overlay_anchor_scope/parent.on");
    let design = fixture_path("overlay_anchor_scope/anchor_scope.one");
    let output =
        run_model_and_format_with_design(&model, Some(&design), Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn overlay_anchor_scope_transitive() {
    // Regression test for a former evaluation-ordering footgun. An overlay on child
    // (`base.c = 2 * multiplier`) references a parent-local parameter whose own RHS
    // depends on an external reference (`multiplier = 2 * src.base`). Under the old
    // phased evaluator, `multiplier` was deferred to Phase 3 (after refs), but the
    // child's overlay RHS was evaluated during the child's Phase 2 setup — triggering
    // a lookup of an unevaluated parent parameter and panicking. Under lazy evaluation
    // the overlay simply forces `multiplier` on demand.
    // Expected: c.base = 2 * (2 * 3) = 12
    let model = fixture_path("overlay_anchor_scope_transitive/parent.on");
    let design = fixture_path("overlay_anchor_scope_transitive/anchor_scope_transitive.one");
    let output =
        run_model_and_format_with_design(&model, Some(&design), Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn design_augmented_override() {
    // Test overriding a design-augmented parameter via scoped syntax (param.ref = value)
    // parent.on has child.doubled = 20 via design augment
    // override_augmented.one tries to set doubled.c = 100
    // Expected: If scoped overrides work for augmented params, doubled.c = 100, else 20
    let model = fixture_path("augmented_refs/parent.on");
    let design = fixture_path("augmented_refs/override_augmented.one");
    let output =
        run_model_and_format_with_design(&model, Some(&design), Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}
