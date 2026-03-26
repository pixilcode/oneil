//! Binary entry point for the Oneil CLI.

#![expect(
    clippy::multiple_crate_versions,
    reason = "this isn't causing problems, and it's going to take time to fix"
)]

#[cfg(feature = "rust-lib")]
fn main() {
    oneil_cli::main();
}

#[cfg(not(feature = "rust-lib"))]
fn main() {
    unimplemented!("Python library does not have a CLI");
}
