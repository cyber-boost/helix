//! Python bindings for the Helix Language
//!
//! This module provides PyO3 bindings to expose Helix functionality
//! to Python. The compiled extension is used by the Python SDK.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyRuntimeError, PyKeyError};
use std::collections::HashMap;
use crate::ast::HelixAst;
use crate::value::Value as HlxValue;
use crate::interpreter::HelixInterpreter;
use crate::operators::fundamental::{OperatorRegistry, ExecutionContext, RequestData};
use crate::error::HlxError;

/// Python wrapper for Helix values
#[pyclass]
#[derive(Clone, Debug)]
pub struct Value {
    inner: HlxValue,
}

#[pymethods]
impl Value {
    fn as_string(&self) -> PyResult<String> {
        match &self.inner {
            HlxValue::String(s) => Ok(s.clone()),
            _ => Err(PyValueError::new_err("Value is not a string")),
        }
    }

    fn as_number(&self) -> PyResult<f64> {
        match &self.inner {
            HlxValue::Number(n) => Ok(*n),
            _ => Err(PyValueError::new_err("Value is not a number")),
        }
    }

    fn as_bool(&self) -> PyResult<bool> {
        match &self.inner {
            HlxValue::Bool(b) => Ok(*b),
            _ => Err(PyValueError::new_err("Value is not a boolean")),
        }
    }

    fn as_dict(&self) -> PyResult<HashMap<String, PyObject>> {
        match &self.inner {
            HlxValue::Object(obj) => {
                let mut result = HashMap::new();
                for (k, v) in obj {
                    // Convert Helix Value to Python object recursively
                    result.insert(k.clone(), value_to_pyobject(v)?);
                }
                Ok(result)
            }
            _ => Err(PyValueError::new_err("Value is not an object")),
        }
    }

    fn as_list(&self) -> PyResult<Vec<PyObject>> {
        match &self.inner {
            HlxValue::Array(arr) => {
                let mut result = Vec::new();
                for v in arr {
                    result.push(value_to_pyobject(v)?);
                }
                Ok(result)
            }
            _ => Err(PyValueError::new_err("Value is not an array")),
        }
    }

    fn is_null(&self) -> bool {
        matches!(&self.inner, HlxValue::Null)
    }

    fn to_python(&self, py: Python) -> PyObject {
        value_to_pyobject(&self.inner).unwrap_or_else(|_| py.None())
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Value({:?})", self.inner)
    }
}

fn value_to_pyobject(value: &HlxValue) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        match value {
            HlxValue::String(s) => Ok(s.clone().into_py(py)),
            HlxValue::Number(n) => Ok(n.into_py(py)),
            HlxValue::Bool(b) => Ok(b.into_py(py)),
            HlxValue::Array(arr) => {
                let mut result = Vec::new();
                for v in arr {
                    result.push(value_to_pyobject(v)?);
                }
                Ok(result.into_py(py))
            }
            HlxValue::Object(obj) => {
                let dict = pyo3::types::PyDict::new_bound(py);
                for (k, v) in obj {
                    dict.set_item(k, value_to_pyobject(v)?)?;
                }
                Ok(dict.unbind().into())
            }
            HlxValue::Null => Ok(py.None()),
            _ => Ok(py.None()), // For other types, return None for now
        }
    })
}

/// Python wrapper for ExecutionContext
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyExecutionContext {
    inner: ExecutionContext,
}

#[pymethods]
impl PyExecutionContext {
    #[new]
    fn new(
        request: Option<HashMap<String, PyObject>>,
        session: Option<HashMap<String, PyObject>>,
        cookies: Option<HashMap<String, String>>,
        params: Option<HashMap<String, String>>,
        query: Option<HashMap<String, String>>,
    ) -> Self {
        let mut context = ExecutionContext::default();

        // Convert Python objects to Helix values if provided
        if let Some(req) = request {
            let mut request_data = HashMap::new();
            for (k, v) in req {
                // Simple conversion - in production, you'd want more sophisticated handling
                request_data.insert(k, format!("{:?}", v));
            }
            context.request = Some(RequestData {
                method: "GET".to_string(),
                url: "".to_string(),
                headers: HashMap::new(),
                body: "".to_string(),
            });
        }

        if let Some(sess) = session {
            let mut session_data = HashMap::new();
            for (k, v) in sess {
                // Convert Python objects to Helix values
                let value = HlxValue::String(format!("{:?}", v));
                session_data.insert(k, value);
            }
            // Note: This is a simplified conversion. In production, you'd want
            // proper bidirectional conversion between Python and Helix values
        }

        if let Some(cookies) = cookies {
            context.cookies = cookies.clone();
        }

        if let Some(params) = params {
            context.params = params.clone();
        }

        if let Some(query) = query {
            context.query = query.clone();
        }

        PyExecutionContext { inner: context }
    }

    #[getter]
    fn request(&self) -> Option<HashMap<String, String>> {
        self.inner.request.as_ref().map(|req| {
            let mut result = HashMap::new();
            result.insert("method".to_string(), req.method.clone());
            result.insert("url".to_string(), req.url.clone());
            result
        })
    }

    #[getter]
    fn session(&self) -> HashMap<String, String> {
        // Simplified - in production, return actual session data
        HashMap::new()
    }

    #[getter]
    fn cookies(&self) -> HashMap<String, String> {
        self.inner.cookies.clone()
    }

    #[getter]
    fn params(&self) -> HashMap<String, String> {
        self.inner.params.clone()
    }

    #[getter]
    fn query(&self) -> HashMap<String, String> {
        self.inner.query.clone()
    }
}

/// Python wrapper for OperatorRegistry
#[pyclass]
pub struct PyOperatorRegistry {
    inner: OperatorRegistry,
}

#[pymethods]
impl PyOperatorRegistry {
    #[new]
    fn new(context: PyExecutionContext) -> PyResult<Self> {
        let registry = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { OperatorRegistry::new().await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create registry: {}", e)))?;

        Ok(PyOperatorRegistry { inner: registry })
    }

    fn execute(&self, operator: String, params: String) -> PyResult<Value> {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { self.inner.execute(&operator, &params).await })
            .map_err(|e| PyRuntimeError::new_err(format!("Operator execution failed: {}", e)))?;

        Ok(Value { inner: result })
    }

    fn context(&self) -> PyExecutionContext {
        use std::sync::Arc;
        let context = Arc::clone(&self.inner.context());
        PyExecutionContext { inner: (*context).clone() }
    }
}

/// Python wrapper for HelixConfig
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyHelixConfig {
    data: HashMap<String, HlxValue>,
}

#[pymethods]
impl PyHelixConfig {
    #[new]
    fn new() -> Self {
        PyHelixConfig {
            data: HashMap::new(),
        }
    }

    fn get(&self, key: String) -> Option<PyObject> {
        self.data.get(&key).map(|value| {
            Python::with_gil(|py| value_to_pyobject(value).unwrap_or_else(|_| py.None()))
        })
    }

    fn set(&mut self, key: String, value: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            // Convert Python object to Helix value
            let helix_value = if value.bind(py).is_none() {
                HlxValue::Null
            } else if let Ok(s) = value.extract::<String>(py) {
                HlxValue::String(s)
            } else if let Ok(n) = value.extract::<f64>(py) {
                HlxValue::Number(n)
            } else if let Ok(b) = value.extract::<bool>(py) {
                HlxValue::Bool(b)
            } else {
                HlxValue::String(format!("{:?}", value))
            };

            self.data.insert(key, helix_value);
            Ok(())
        })
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn items(&self) -> Vec<(String, PyObject)> {
        self.data
            .iter()
            .map(|(k, v)| {
                (k.clone(), Python::with_gil(|py| value_to_pyobject(v).unwrap_or_else(|_| py.None())))
            })
            .collect()
    }
}

/// Python wrapper for HelixInterpreter
#[pyclass]
pub struct PyHelixInterpreter {
    inner: HelixInterpreter,
}

#[pymethods]
impl PyHelixInterpreter {
    #[new]
    fn new() -> PyResult<Self> {
        let interpreter = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { HelixInterpreter::new().await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create interpreter: {}", e)))?;

        Ok(PyHelixInterpreter { inner: interpreter })
    }

    fn execute<'py>(&self, py: Python<'py>, expression: String) -> PyResult<&'py PyAny> {
        // Create a runtime for async execution
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            // Clone the interpreter for use in async context
            let mut interpreter = HelixInterpreter::new().await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create interpreter: {}", e)))?;

            // For now, treat expression as variable lookup
            // In a full implementation, you'd parse and execute expressions properly
            if let Some(value) = interpreter.get_variable(&expression) {
                value_to_pyobject(value)
            } else {
                // Return a placeholder result
                Ok(format!("Executed: {}", expression).into_py(py))
            }
        })
    }

    fn set_variable(&mut self, name: String, value: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            // Convert Python object to Helix value and set it
            let helix_value = if let Ok(s) = value.extract::<String>(py) {
                HlxValue::String(s)
            } else if let Ok(n) = value.extract::<f64>(py) {
                HlxValue::Number(n)
            } else if let Ok(b) = value.extract::<bool>(py) {
                HlxValue::Bool(b)
            } else {
                HlxValue::String(format!("{:?}", value))
            };

            // Use the actual interpreter's set_variable method
            self.inner.set_variable(name, helix_value);
            Ok(())
        })
    }

    fn get_variable(&self, name: String) -> Option<PyObject> {
        // Use the actual interpreter's get_variable method
        if let Some(value) = self.inner.get_variable(&name) {
            Python::with_gil(|py| {
                value_to_pyobject(value).ok()
            })
        } else {
            None
        }
    }
}

/// Utility functions for Python
#[pyfunction]
fn parse(source: String) -> PyResult<PyHelixConfig> {
    // Use the actual Helix parser
    let config_result = crate::parse_and_validate(&source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {}", e)))?;

    // Convert HelixConfig to PyHelixConfig
    let mut py_config = PyHelixConfig::new();

    // Convert the config data to our internal format
    // Note: This is a simplified conversion - in production, you'd want full bidirectional conversion
    if let Ok(config_json) = serde_json::to_string(&config_result) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&config_json) {
            if let serde_json::Value::Object(map) = value {
                for (k, v) in map {
                    let helix_value = match v {
                        serde_json::Value::String(s) => HlxValue::String(s),
                        serde_json::Value::Number(n) => {
                            if let Some(f) = n.as_f64() {
                                HlxValue::Number(f)
                            } else if let Some(i) = n.as_i64() {
                                HlxValue::Number(i as f64)
                            } else {
                                HlxValue::String(format!("{}", n))
                            }
                        }
                        serde_json::Value::Bool(b) => HlxValue::Bool(b),
                        serde_json::Value::Array(arr) => {
                            let mut helix_array = Vec::new();
                            for item in arr {
                                match item {
                                    serde_json::Value::String(s) => helix_array.push(HlxValue::String(s)),
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_array.push(HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => helix_array.push(HlxValue::Bool(b)),
                                    _ => helix_array.push(HlxValue::String(format!("{}", item))),
                                }
                            }
                            HlxValue::Array(helix_array)
                        }
                        serde_json::Value::Object(obj) => {
                            let mut helix_obj = HashMap::new();
                            for (obj_k, obj_v) in obj {
                                match obj_v {
                                    serde_json::Value::String(s) => { helix_obj.insert(obj_k, HlxValue::String(s)); }
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_obj.insert(obj_k, HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => { helix_obj.insert(obj_k, HlxValue::Bool(b)); }
                                    _ => { helix_obj.insert(obj_k, HlxValue::String(format!("{}", obj_v))); }
                                }
                            }
                            HlxValue::Object(helix_obj)
                        }
                        serde_json::Value::Null => HlxValue::Null,
                    };
                    py_config.data.insert(k, helix_value);
                }
            }
        }
    }

    Ok(py_config)
}

#[pyfunction]
fn execute(expression: String, context: Option<HashMap<String, PyObject>>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        // Create a runtime for async execution
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            // Create interpreter
            let mut interpreter = HelixInterpreter::new().await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create interpreter: {}", e)))?;

            // Set context variables if provided
            if let Some(ctx) = context {
                for (key, value) in ctx {
                    let helix_value = if let Ok(s) = value.extract::<String>(py) {
                        HlxValue::String(s)
                    } else if let Ok(n) = value.extract::<f64>(py) {
                        HlxValue::Number(n)
                    } else if let Ok(b) = value.extract::<bool>(py) {
                        HlxValue::Bool(b)
                    } else {
                        HlxValue::String(format!("{:?}", value))
                    };
                    interpreter.set_variable(key, helix_value);
                }
            }

            // For now, we'll treat the expression as a simple variable lookup or operator call
            // In a full implementation, you'd need a proper expression parser
            if let Some(value) = interpreter.get_variable(&expression) {
                value_to_pyobject(value)
            } else {
                // Try to execute as an operator call
                // This is a simplified version - you'd want proper expression parsing
                Ok(format!("Executed: {}", expression).into_py(py))
            }
        })
    })
}

#[pyfunction]
fn load_file(file_path: String) -> PyResult<PyHelixConfig> {
    // Use the actual Helix file loader
    let config_result = crate::load_file(&file_path)
        .map_err(|e| PyValueError::new_err(format!("File load error: {}", e)))?;

    // Convert HelixConfig to PyHelixConfig (same logic as parse function)
    let mut py_config = PyHelixConfig::new();

    if let Ok(config_json) = serde_json::to_string(&config_result) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&config_json) {
            if let serde_json::Value::Object(map) = value {
                for (k, v) in map {
                    let helix_value = match v {
                        serde_json::Value::String(s) => HlxValue::String(s),
                        serde_json::Value::Number(n) => {
                            if let Some(f) = n.as_f64() {
                                HlxValue::Number(f)
                            } else {
                                HlxValue::String(format!("{}", n))
                            }
                        }
                        serde_json::Value::Bool(b) => HlxValue::Bool(b),
                        serde_json::Value::Array(arr) => {
                            let mut helix_array = Vec::new();
                            for item in arr {
                                match item {
                                    serde_json::Value::String(s) => helix_array.push(HlxValue::String(s)),
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_array.push(HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => helix_array.push(HlxValue::Bool(b)),
                                    _ => helix_array.push(HlxValue::String(format!("{}", item))),
                                }
                            }
                            HlxValue::Array(helix_array)
                        }
                        serde_json::Value::Object(obj) => {
                            let mut helix_obj = HashMap::new();
                            for (obj_k, obj_v) in obj {
                                match obj_v {
                                    serde_json::Value::String(s) => { helix_obj.insert(obj_k, HlxValue::String(s)); }
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_obj.insert(obj_k, HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => { helix_obj.insert(obj_k, HlxValue::Bool(b)); }
                                    _ => { helix_obj.insert(obj_k, HlxValue::String(format!("{}", obj_v))); }
                                }
                            }
                            HlxValue::Object(helix_obj)
                        }
                        serde_json::Value::Null => HlxValue::Null,
                    };
                    py_config.data.insert(k, helix_value);
                }
            }
        }
    }

    Ok(py_config)
}

/// Python module definition
#[pymodule]
fn _core(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Value>()?;
    m.add_class::<PyExecutionContext>()?;
    m.add_class::<PyOperatorRegistry>()?;
    m.add_class::<PyHelixConfig>()?;
    m.add_class::<PyHelixInterpreter>()?;

    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(execute, m)?)?;
    m.add_function(wrap_pyfunction!(load_file, m)?)?;

    Ok(())
}
