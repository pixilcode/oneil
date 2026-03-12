use indexmap::IndexMap;
use oneil_shared::symbols::PyFunctionName;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

#[derive(Debug, Default)]
pub struct PythonFunctionMap {
    entries: IndexMap<PyFunctionName, PythonFunction>,
}

impl PythonFunctionMap {
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    pub fn get_function(&self, identifier: &PyFunctionName) -> Option<&PythonFunction> {
        self.entries.get(identifier)
    }

    pub fn get_function_names(&self) -> impl Iterator<Item = &PyFunctionName> {
        self.entries.keys()
    }
}

impl From<IndexMap<PyFunctionName, PythonFunction>> for PythonFunctionMap {
    fn from(entries: IndexMap<PyFunctionName, PythonFunction>) -> Self {
        Self { entries }
    }
}

impl From<PythonFunctionMap> for IndexMap<PyFunctionName, PythonFunction> {
    fn from(map: PythonFunctionMap) -> Self {
        map.entries
    }
}

#[derive(Debug)]
pub struct PythonFunction {
    function: Py<PyAny>,
}

impl PythonFunction {
    pub const fn new(function: Py<PyAny>) -> Self {
        Self { function }
    }

    /// Calls the Python function with the given positional arguments.
    pub fn call<'py>(
        &self,
        py: Python<'py>,
        args: &[Bound<'py, PyAny>],
    ) -> PyResult<Bound<'py, PyAny>> {
        let callable = self.function.bind(py);
        let args_tuple = PyTuple::new(py, args)?;
        callable.call1(args_tuple)
    }
}
