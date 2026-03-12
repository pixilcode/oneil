//! Path types for model and Python module locations.

use std::path::{Path, PathBuf};

/// The extension for Oneil source files.
const ON_EXTENSION: &str = "on";

/// The extension for Python module files.
const PYTHON_EXTENSION: &str = "py";

/// A path to an Oneil model file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelPath(PathBuf);

impl ModelPath {
    /// Creates a new model path from a path with the `.on` extension.
    ///
    /// # Panics
    ///
    /// Panics if the path has an extension other than `.on`.
    #[must_use]
    fn new(path: PathBuf) -> Self {
        debug_assert_eq!(
            path.extension().map(|ext| ext.to_string_lossy()),
            Some(ON_EXTENSION.into()),
            "Model paths must have an extension of .{ON_EXTENSION}"
        );

        Self(path)
    }

    /// Creates a new model path from a string without the extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path has an extension.
    #[must_use]
    pub fn from_str_no_ext(s: &str) -> Self {
        let path = PathBuf::from(s);

        assert_eq!(
            path.extension(),
            None,
            "given path must not have an extension, got {}",
            path.display()
        );

        Self(path.with_extension(ON_EXTENSION))
    }

    /// Creates a new model path from a path with the `.on` extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path does not have the `.on` extension.
    #[must_use]
    pub fn from_src_with_ext(path: &str) -> Self {
        Self::from_path_with_ext(Path::new(path))
    }

    /// Creates a new model path from a path without the extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path has an extension.
    #[must_use]
    pub fn from_path_no_ext(path: &Path) -> Self {
        let path = path.to_path_buf();

        assert_eq!(
            path.extension(),
            None,
            "given path must not have an extension, got {}",
            path.display()
        );

        Self(path.with_extension(ON_EXTENSION))
    }

    /// Creates a new model path from a path with the `.on` extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path does not have the `.on` extension.
    #[must_use]
    pub fn from_path_with_ext(path: &Path) -> Self {
        let path = path.to_path_buf();

        assert_eq!(
            path.extension().map(|ext| ext.to_string_lossy()),
            Some(ON_EXTENSION.into()),
            "given path must have `.{ON_EXTENSION}` extension, got {}",
            path.display()
        );

        Self(path.with_extension(ON_EXTENSION))
    }

    /// Returns the path as a reference.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Returns the underlying path buffer.
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }

    /// Returns a path for a sibling model relative to the current model's path
    /// with a `.on` extension.
    #[must_use]
    pub fn get_sibling_model_path(&self, sibling_path: Self) -> Self {
        let parent = self.0.parent();
        let sibling_path = sibling_path.into_path_buf();

        if let Some(parent) = parent {
            Self::new(parent.join(sibling_path))
        } else {
            Self::new(sibling_path)
        }
    }

    /// Returns a path for a sibling Python module relative to the current model's path.
    #[must_use]
    pub fn get_sibling_python_path(&self, sibling_path: PythonPath) -> PythonPath {
        let parent = self.0.parent();
        let sibling_path = sibling_path.into_path_buf();

        if let Some(parent) = parent {
            PythonPath::new(parent.join(sibling_path))
        } else {
            PythonPath::new(sibling_path)
        }
    }
}

/// A path to a Python module file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PythonPath(PathBuf);

impl PythonPath {
    fn new(path: PathBuf) -> Self {
        assert_eq!(
            path.extension().map(|ext| ext.to_string_lossy()),
            Some(PYTHON_EXTENSION.into()),
            "Python paths must have an extension of .{PYTHON_EXTENSION}"
        );

        Self(path)
    }

    /// Creates a new Python path from a string without the extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path has an extension.
    #[must_use]
    pub fn from_str_no_ext(s: &str) -> Self {
        let path = PathBuf::from(s);

        debug_assert_eq!(
            path.extension(),
            None,
            "given path must not have an extension, got {}",
            path.display()
        );

        Self(path.with_extension(PYTHON_EXTENSION))
    }

    /// Creates a new Python path from a path without the extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path has an extension.
    #[must_use]
    pub fn from_path_no_ext(path: &Path) -> Self {
        let path = path.to_path_buf();

        assert_eq!(
            path.extension(),
            None,
            "given path must not have an extension, got {}",
            path.display()
        );

        Self(path.with_extension(PYTHON_EXTENSION))
    }

    /// Returns the path as a reference.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Returns the underlying path buffer.
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

/// A path to a source file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourcePath(PathBuf);

impl SourcePath {
    /// Creates a new source path from a path with the `.on` extension.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Returns the path as a reference.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Returns the underlying path buffer.
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl From<&ModelPath> for SourcePath {
    fn from(value: &ModelPath) -> Self {
        Self::new(value.clone().into_path_buf())
    }
}

impl From<&PythonPath> for SourcePath {
    fn from(value: &PythonPath) -> Self {
        Self::new(value.clone().into_path_buf())
    }
}
