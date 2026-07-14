//! Load a Python virtual environment by finding or using a provided venv path.
//!
//! When the `python` feature is enabled, the CLI can automatically discover
//! a `venv` or `.venv` directory in the current path or its parents and prepend
//! its `bin` (or `Scripts` on Windows) directory to `PATH`.
//!
//! This allows the CLI to use the Python interpreter and libraries installed in
//! the venv, regardless of whether the user has activated the venv in their shell.
//!
//! This is especially useful for the LSP, which the VS Code or Cursor usually run
//! without a venv activated.

use std::env;
use std::path::{Path, PathBuf};

const VENV_DIR_NAMES: &[&str] = &["venv", ".venv"];

/// Ensures a Python virtual environment is on `PATH`.
///
/// - If `venv_path_override` is `Some(path)`, that path is treated as the venv root
///   and its `bin` (or `Scripts` on Windows) directory is prepended to `PATH`.
///   This is done regardless of whether `VIRTUAL_ENV` is set.
/// - Otherwise, if `VIRTUAL_ENV` is set, nothing is done (the environment is
///   already using a venv).
/// - Otherwise, [`try_find_venv`] is used to search upward for a `venv` or `.venv`
///   directory; if one is found, its executable directory is prepended to `PATH`.
pub fn try_load_venv(venv_path_override: Option<&Path>) {
    if let Some(venv_root) = venv_path_override {
        prepend_venv_bin_to_path(venv_root);
        return;
    }

    if env::var_os("VIRTUAL_ENV").is_some() {
        return;
    }

    if let Some(venv_root) = try_find_venv() {
        prepend_venv_bin_to_path(&venv_root);
    }
}

/// Traverses the directory tree upwards from the current working directory,
/// looking for a direct child directory named `venv` or `.venv`.
///
/// Returns the first such directory found, or `None` if none is found before
/// reaching the filesystem root.
///
/// NOTE: This should only be called at CLI startup before any other threads
/// are spawned because it uses `unsafe` to set the `PATH` environment variable.
///
/// # Errors
///
/// Returns `None` if the current directory cannot be determined.
pub fn try_find_venv() -> Option<PathBuf> {
    let current = env::current_dir().ok()?;
    let mut dir = current.as_path();

    loop {
        for name in VENV_DIR_NAMES {
            let candidate = dir.join(name);

            if candidate.is_dir() {
                return Some(candidate);
            }
        }

        dir = dir.parent()?;
    }
}

/// Directory name for the venv executables (e.g. `bin` on Unix, `Scripts` on Windows).
const fn venv_bin_dir_name() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}

/// Prepends the venv's executable directory to the process `PATH` environment variable.
fn prepend_venv_bin_to_path(venv_root: &Path) {
    let bin_dir = venv_root.join(venv_bin_dir_name());

    if !bin_dir.is_dir() {
        return;
    }

    let existing = env::var_os("PATH").unwrap_or_default();
    let mut components: Vec<PathBuf> = env::split_paths(&existing).collect();

    components.insert(0, bin_dir);
    if let Ok(new_path) = env::join_paths(components) {
        // SAFETY: Called once at CLI startup before any other threads are spawned.
        unsafe {
            env::set_var("PATH", new_path);
        }
    }
}
