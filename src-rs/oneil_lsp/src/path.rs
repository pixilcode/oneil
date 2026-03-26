//! Workspace-relative path display helpers.

use std::path::{Path, PathBuf};

/// Returns a display string for `path`, stripping a matching workspace root prefix when present.
///
/// Workspace roots are matched longest-first so a nested folder wins over its parent. If `path`
/// equals a root, returns `"."`. If no root is a prefix of `path`, returns `path.display().to_string()`.
#[must_use]
pub fn trim_path<'path>(path: &'path Path, workspace_roots: &[PathBuf]) -> Option<&'path Path> {
    let mut roots: Vec<&Path> = workspace_roots.iter().map(PathBuf::as_path).collect();
    roots.sort_by_key(|root| std::cmp::Reverse(root.components().count()));

    for root in roots {
        if let Ok(suffix) = path.strip_prefix(root) {
            return if suffix.as_os_str().is_empty() {
                Some(Path::new("."))
            } else {
                Some(suffix)
            };
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_path_strips_longest_workspace_root() {
        let roots = vec![PathBuf::from("/ws"), PathBuf::from("/ws/nested")];
        let path = Path::new("/ws/nested/pkg/model.on");
        assert_eq!(trim_path(path, &roots), Some(Path::new("pkg/model.on")));
    }

    #[test]
    fn trim_path_root_is_dot() {
        let roots = vec![PathBuf::from("/ws/proj")];
        let path = Path::new("/ws/proj");
        assert_eq!(trim_path(path, &roots), Some(Path::new(".")));
    }

    #[test]
    fn trim_path_unmatched_unchanged() {
        let roots = vec![PathBuf::from("/other")];
        let path = Path::new("/ws/a.on");
        assert_eq!(trim_path(path, &roots), None);
    }
}
