//! Snapshot testing for Oneil evaluation output and errors.
//!
//! This crate provides integration-style snapshot tests that run the full
//! Oneil pipeline (parse -> resolve -> eval) and capture evaluation output
//! and errors in a canonical format for comparison.

#[cfg(test)]
mod test;

#[cfg(test)]
mod util {
    use std::{fmt::Write, path::Path};

    use oneil_runtime::{
        Runtime,
        output::{self, OneilError},
    };
    use oneil_shared::paths::{DesignPath, ModelPath};

    /// Runs the full evaluation pipeline on an Oneil model file and returns
    /// a formatted string containing any errors and the evaluation output.
    ///
    /// The output format is deterministic and suitable for snapshot testing:
    /// errors are listed first (if any), then a separator, then the model
    /// output (tests and parameters).
    ///
    /// Paths in the output are normalized by stripping `path_prefix` when present,
    /// so that snapshots are portable (e.g. use `CARGO_MANIFEST_DIR` as the prefix).
    ///
    /// # Errors
    ///
    /// This function does not return a `Result`; parse, resolution, and
    /// evaluation errors are included in the returned string.
    #[must_use]
    pub fn run_model_and_format(path: &Path, path_prefix: Option<&Path>) -> String {
        run_model_and_format_with_design(path, None, path_prefix)
    }

    /// Runs the full evaluation pipeline on an Oneil model file with an optional
    /// design file applied, and returns a formatted string containing any errors
    /// and the evaluation output.
    ///
    /// When a design path is provided, the design file's parameter overrides are
    /// applied to the model being evaluated (simulating the CLI `--design` flag).
    #[expect(clippy::unwrap_used, reason = "writing to a String is infallible")]
    #[must_use]
    pub fn run_model_and_format_with_design(
        path: &Path,
        design_path: Option<&Path>,
        path_prefix: Option<&Path>,
    ) -> String {
        let path = ModelPath::from_path_with_ext(path);
        let design_path = design_path.map(DesignPath::from_path_with_ext);

        let mut runtime = Runtime::new();
        let (model_opt, errors) = runtime.eval_model(&path, design_path.as_ref());

        let mut out = String::new();

        let errors_str = format_errors(errors.to_vec(), path_prefix);
        if !errors_str.is_empty() {
            writeln!(out, "{errors_str}").unwrap();
        }

        if let Some(model_ref) = model_opt {
            let model_str = format_model(model_ref, path_prefix);
            if !out.is_empty() {
                writeln!(out, "---\n").unwrap();
            }
            write!(out, "{model_str}").unwrap();
        }

        if out.is_empty() {
            write!(out, "(no output)").unwrap();
        }

        out
    }

    /// Returns a path string normalized for snapshots: if it starts with `prefix`, strip it.
    fn normalize_path(path: &Path, prefix: Option<&Path>) -> String {
        let path_str = path.display().to_string();

        let prefix = match prefix {
            Some(p) => p.display().to_string(),
            None => return path_str,
        };

        if path_str.starts_with(&prefix) {
            path_str[prefix.len()..]
                .trim_start_matches(std::path::MAIN_SEPARATOR)
                .to_string()
        } else {
            path_str
        }
    }

    /// Formats a collection of Oneil errors into a canonical string for snapshots.
    fn format_errors(errors: Vec<&OneilError>, path_prefix: Option<&Path>) -> String {
        errors
            .into_iter()
            .filter(|e| !e.is_internal_error())
            .map(|e| format_error(e, path_prefix))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Formats a single error as a stable, readable string.
    #[expect(clippy::unwrap_used, reason = "writing to a String is infallible")]
    fn format_error(error: &OneilError, path_prefix: Option<&Path>) -> String {
        let path_str = normalize_path(error.path(), path_prefix);

        let loc = error
            .location()
            .map(|l| format!("{}:{}", l.line(), l.column()));

        let at = loc
            .as_deref()
            .map_or_else(|| path_str.clone(), |loc| format!("{path_str}:{loc}"));

        let message = normalize_message(error.message(), path_prefix);
        let mut out = format!("error: {message}\n  at {at}");

        for ctx in error.context() {
            let (kind, text) = match ctx {
                oneil_shared::error::Context::Note(msg) => {
                    ("note", normalize_message(msg, path_prefix))
                }
                oneil_shared::error::Context::Help(msg) => {
                    ("help", normalize_message(msg, path_prefix))
                }
            };
            write!(out, "\n  {kind}: {text}").unwrap();
        }

        out
    }

    /// Strips occurrences of `prefix` from anywhere in `message`, so
    /// diagnostic strings that embed absolute paths (e.g. cycle chains
    /// of compilation units) render portably across machines.
    fn normalize_message(message: &str, prefix: Option<&Path>) -> String {
        let Some(prefix) = prefix else {
            return message.to_string();
        };
        let mut prefix_str = prefix.display().to_string();
        if !prefix_str.ends_with(std::path::MAIN_SEPARATOR) {
            prefix_str.push(std::path::MAIN_SEPARATOR);
        }
        message.replace(&prefix_str, "")
    }

    /// Formats an evaluated model's output (tests and parameters) for snapshots.
    #[expect(clippy::unwrap_used, reason = "writing to a String is infallible")]
    fn format_model(
        model_ref: output::reference::ModelReference<'_>,
        path_prefix: Option<&Path>,
    ) -> String {
        let mut out = String::new();

        let path = normalize_path(model_ref.path().as_path(), path_prefix);
        let tests = model_ref.tests();
        let passed = tests.iter().filter(|(_, test)| test.passed()).count();
        let total = tests.len();

        writeln!(out, "Model: {path}").unwrap();
        writeln!(out, "Tests: {passed}/{total}").unwrap();

        for (index, test) in tests {
            let result_str = if test.passed() { "PASS" } else { "FAIL" };
            writeln!(out, "  test {}: {result_str}", index.into_usize() + 1).unwrap();
        }

        let params = model_ref.parameters();
        if !params.is_empty() {
            out.push_str("Parameters:\n");
            for (name, param) in params {
                let prefix = match param.print_level {
                    output::PrintLevel::Performance => "$ ",
                    output::PrintLevel::Trace => "* ",
                    output::PrintLevel::None => "",
                };
                let value_str = format_value(&param.value);
                writeln!(out, "  {prefix}{name} = {value_str}").unwrap();
            }
        }

        out
    }

    /// Formats a value for snapshot output (deterministic, no colors).
    fn format_value(value: &output::Value) -> String {
        format!("{value:?}")
    }
}
