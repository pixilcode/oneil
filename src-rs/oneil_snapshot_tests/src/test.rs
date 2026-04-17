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
    // Both refs share the same instance, overlay affects both
    // Expected: s = 99 + 99 = 198
    let path = fixture_path("design_overlay/shared_ref.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn overlay_two_instances() {
    // `use` creates unique instances, overlay affects only one
    // Expected: s = 99 + 10 = 109
    let path = fixture_path("design_overlay/two_instances.on");
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
    // with [inner] clause without replacement
    // Expected: s = 10 (from design_child's x)
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
