//! Loading Python code and extracting callables.

use std::ffi::CString;

use indexmap::IndexMap;
use oneil_shared::{paths::PythonPath, symbols::PyFunctionName};
use pyo3::{prelude::*, types::PyDict, wrap_pymodule};

use crate::{
    error::LoadPythonImportError,
    function::{PythonFunction, PythonModule},
    py_compat::oneil_python_module,
};

pub fn load_python_import(
    path: &PythonPath,
    source: &str,
) -> Result<PythonModule, LoadPythonImportError> {
    // get the module name from the path
    let path = path.as_path().to_string_lossy();
    let module_name = path.trim_end_matches(".py").replace('/', ".");

    // convert the path and module name to C strings
    let path_cstr = CString::new(path.as_bytes()).expect("path should not have a null byte");
    let module_name_cstr =
        CString::new(module_name).expect("module name should not have a null byte");

    // convert the source to a C string
    let source_cstr = match CString::new(source) {
        Ok(cstr) => cstr,
        Err(_null_error) => return Err(LoadPythonImportError::SourceHasNullByte),
    };

    let functions = Python::attach(|py| {
        insert_oneil_module_into_python(py)?;

        // load the code module
        let code_module = PyModule::from_code(py, &source_cstr, &path_cstr, &module_name_cstr)?;

        // get the inspect module
        let inspect_module = PyModule::import(py, "inspect")?;

        // get the functions from the code module
        let functions = code_module
            .dict()
            .iter()
            .filter(|(key, value)| !key.to_string().starts_with("__") && value.is_callable())
            .map(|(key, value)| {
                let name = key.to_string();
                let docs = get_doc_string(&value, &inspect_module);
                let line_no = get_line_no(&value, &inspect_module);
                let value = value.unbind();
                Ok((
                    PyFunctionName::from(name),
                    PythonFunction::new(value, docs, line_no),
                ))
            })
            .collect::<PyResult<IndexMap<_, _>>>()?;

        let module_docs = get_doc_string(&code_module, &inspect_module);

        Ok::<_, PyErr>((module_docs, functions))
    });

    // return the functions
    match functions {
        Ok((module_docs, functions)) => Ok(PythonModule::new(module_docs, functions)),
        Err(e) => Err(LoadPythonImportError::CouldNotLoadPythonModule(e)),
    }
}

fn insert_oneil_module_into_python(py: Python<'_>) -> PyResult<()> {
    // wrap the oneil_python_module into a Python module
    let oneil_module = wrap_pymodule!(oneil_python_module)(py);

    // Import and get sys.modules
    let sys = PyModule::import(py, "sys")?;
    let py_modules: Bound<'_, PyDict> = sys.getattr("modules")?.cast_into()?;

    // Insert oneil_python_module into sys.modules
    py_modules.set_item("oneil", oneil_module)?;

    Ok(())
}

fn get_doc_string(
    value: &Bound<'_, PyAny>,
    inspect_module: &Bound<'_, PyModule>,
) -> Option<String> {
    inspect_module
        .call_method1("getdoc", (value,))
        .expect("getdoc should not fail")
        .extract::<Option<String>>()
        .expect("getdoc should return either a string or None")
}

fn get_line_no(value: &Bound<'_, PyAny>, inspect_module: &Bound<'_, PyModule>) -> Option<u32> {
    let result = inspect_module
        .call_method1("getsourcelines", (value,))
        // if the call fails, it was probably because
        // the source code could not be retrieved or the
        // function is a builtin function
        .ok()?;

    let (_, line_no) = result
        .extract::<(Vec<String>, u32)>()
        .expect("`getsourcelines` should return a tuple of a string and a u32");

    Some(line_no)
}
