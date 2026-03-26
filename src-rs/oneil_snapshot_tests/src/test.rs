//! Integration snapshot tests for Oneil evaluation output and errors.
//!
//! Each test runs the full pipeline (parse → resolve → eval) on a fixture
//! and compares the formatted output against a stored snapshot.

use std::path::PathBuf;

use crate::util::run_model_and_format;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn basic_model_snapshot() {
    let path = fixture_path("basic.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn syntax_error_snapshot() {
    let path = fixture_path("syntax_error.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}

#[test]
fn failing_test_snapshot() {
    let path = fixture_path("failing_test.on");
    let output = run_model_and_format(&path, Some(manifest_dir().as_path()));
    insta::assert_snapshot!(output);
}
