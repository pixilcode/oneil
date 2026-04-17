//! Path types for model and Python module locations.

use std::convert::TryFrom;
use std::path::{Path, PathBuf};

/// The extension for ordinary Oneil model files.
const ON_EXTENSION: &str = "on";

/// The extension for Oneil design bundle files (`use design` targets).
const ONE_EXTENSION: &str = "one";

#[must_use]
fn is_oneil_model_path_extension(ext: Option<&std::ffi::OsStr>) -> bool {
    ext.and_then(|e| e.to_str())
        .is_some_and(|s| s == ON_EXTENSION || s == ONE_EXTENSION)
}

/// The extension for Python module files.
const PYTHON_EXTENSION: &str = "py";

/// A path to an Oneil model or design source file (`.on` or `.one`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelPath(PathBuf);

impl ModelPath {
    /// Creates a new model path from a path with a supported extension (`.on` or `.one`).
    ///
    /// # Panics
    ///
    /// Panics if the path does not use a supported Oneil source extension.
    #[must_use]
    fn new(path: PathBuf) -> Self {
        debug_assert!(
            is_oneil_model_path_extension(path.extension()),
            "Model paths must end with `.{ON_EXTENSION}` or `.{ONE_EXTENSION}`"
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
        Self::from_path_no_ext(Path::new(s))
    }

    /// Creates a new model path from a path with a supported extension (`.on` or `.one`).
    ///
    /// # Panics
    ///
    /// Panics if the given path does not have a supported Oneil source extension.
    #[must_use]
    pub fn from_str_with_ext(path: &str) -> Self {
        Self::from_path_with_ext(Path::new(path))
    }

    /// Creates a new model path from a path without the extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path has an extension.
    #[must_use]
    pub fn from_path_no_ext(path: &Path) -> Self {
        assert!(
            path.extension().is_none(),
            "given path must not have an extension, got {}",
            path.display()
        );

        Self::try_from(path.with_extension(ON_EXTENSION).as_path()).unwrap_or_else(|()| {
            panic!(
                "given path must not have an extension, got {}",
                path.display()
            )
        })
    }

    /// Creates a new model path from a path with a supported extension (`.on` or `.one`).
    ///
    /// # Panics
    ///
    /// Panics if the given path does not have a supported Oneil source extension.
    #[must_use]
    pub fn from_path_with_ext(path: &Path) -> Self {
        Self::try_from(path).unwrap_or_else(|()| {
            panic!(
                "given path must have `.{ON_EXTENSION}` or `.{ONE_EXTENSION}` extension, got {}",
                path.display()
            )
        })
    }

    /// Returns `true` when this path uses the `.one` design bundle extension.
    #[must_use]
    pub fn is_design_bundle(&self) -> bool {
        self.0
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|s| s == ONE_EXTENSION)
    }

    /// Returns a path for a sibling design file relative to the current model's path
    /// with a `.one` extension.
    ///
    /// This is similar to `get_sibling_model_path` but for design files.
    ///
    /// # Panics
    ///
    /// Panics if the resulting path does not have a `.one` extension.
    #[must_use]
    pub fn get_sibling_design_path(&self, sibling_path: Self) -> Self {
        let parent = self.0.parent();
        let sibling_path = sibling_path.into_path_buf().with_extension(ONE_EXTENSION);

        let joined = if let Some(parent) = parent {
            parent.join(&sibling_path)
        } else {
            sibling_path
        };

        Self::try_from(joined.as_path()).unwrap_or_else(|()| {
            panic!(
                "expected design path with `.{ONE_EXTENSION}` extension, got {}",
                joined.display()
            )
        })
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

impl TryFrom<&Path> for ModelPath {
    type Error = ();

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if is_oneil_model_path_extension(path.extension()) {
            Ok(Self(path.to_path_buf()))
        } else {
            Err(())
        }
    }
}

impl TryFrom<&str> for ModelPath {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(Path::new(value))
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

    /// Creates a new Python path from a path with the `.py` extension.
    ///
    /// # Panics
    ///
    /// Panics if the given path does not have the `.py` extension.
    #[must_use]
    pub fn from_path_with_ext(path: &Path) -> Self {
        let path = path.to_path_buf();

        assert_eq!(
            path.extension().map(|ext| ext.to_string_lossy()),
            Some(PYTHON_EXTENSION.into()),
            "Python path must have `.{PYTHON_EXTENSION}` extension, got {}",
            path.display()
        );

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

impl TryFrom<SourcePath> for ModelPath {
    type Error = ();

    /// Attempts to convert a [`SourcePath`] to a [`ModelPath`].
    ///
    /// Succeeds when the path has the `.on` or `.one` extension.
    fn try_from(value: SourcePath) -> Result<Self, Self::Error> {
        if is_oneil_model_path_extension(value.as_path().extension()) {
            Ok(Self::from_path_with_ext(value.as_path()))
        } else {
            Err(())
        }
    }
}

impl TryFrom<SourcePath> for PythonPath {
    type Error = ();

    /// Attempts to convert a [`SourcePath`] to a [`PythonPath`].
    ///
    /// Succeeds only when the path has the `.py` extension.
    fn try_from(value: SourcePath) -> Result<Self, Self::Error> {
        value
            .as_path()
            .extension()
            .filter(|ext| ext.to_string_lossy().as_ref() == PYTHON_EXTENSION)
            .map(|_| Self::from_path_with_ext(value.as_path()))
            .ok_or(())
    }
}
