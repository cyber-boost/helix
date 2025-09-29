use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyRuntimeError};
use std::collections::HashMap;
use crate::dna::atp::value::Value as HlxValue;
use crate::dna::atp::interpreter::HelixInterpreter;
use crate::dna::ops::fundamental::{OperatorRegistry, ExecutionContext, RequestData};
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
        matches!(& self.inner, HlxValue::Null)
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
fn types_value_to_pyobject(value: &crate::dna::atp::types::Value) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        match value {
            crate::dna::atp::types::Value::String(s) => Ok(s.clone().into_py(py)),
            crate::dna::atp::types::Value::Number(n) => Ok(n.into_py(py)),
            crate::dna::atp::types::Value::Bool(b) => Ok(b.into_py(py)),
            crate::dna::atp::types::Value::Array(arr) => {
                let mut result = Vec::new();
                for v in arr {
                    result.push(types_value_to_pyobject(v)?);
                }
                Ok(result.into_py(py))
            }
            crate::dna::atp::types::Value::Object(obj) => {
                let dict = pyo3::types::PyDict::new_bound(py);
                for (k, v) in obj {
                    dict.set_item(k, types_value_to_pyobject(v)?)?;
                }
                Ok(dict.unbind().into())
            }
            crate::dna::atp::types::Value::Null => Ok(py.None()),
            _ => Ok(py.None()),
        }
    })
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
            _ => Ok(py.None()),
        }
    })
}
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
        if let Some(req) = request {
            let mut request_data = HashMap::new();
            for (k, v) in req {
                request_data.insert(k, format!("{:?}", v));
            }
            context.request = Some(RequestData {
                method: "GET".to_string(),
                url: "".to_string(),
                headers: HashMap::new(),
                body: "".to_string(),
            });
        }
        if let Some(session) = session {
            // Handle session data if needed
            for (k, v) in session {
                // Store session data in context if needed
                let _ = (k, v);
            }
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
        PyExecutionContext {
            inner: context,
        }
    }
    #[getter]
    fn request(&self) -> Option<HashMap<String, String>> {
        self.inner
            .request
            .as_ref()
            .map(|req| {
                let mut result = HashMap::new();
                result.insert("method".to_string(), req.method.clone());
                result.insert("url".to_string(), req.url.clone());
                result
            })
    }
    #[getter]
    fn session(&self) -> HashMap<String, String> {
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
#[pyclass]
pub struct PyOperatorRegistry {
    inner: OperatorRegistry,
}
#[pymethods]
impl PyOperatorRegistry {
    #[new]
    fn new(context: PyExecutionContext) -> PyResult<Self> {
        let _ = context; // Use context parameter to avoid warning
        let registry = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { OperatorRegistry::new().await })
            .map_err(|e| PyRuntimeError::new_err(
                format!("Failed to create registry: {}", e),
            ))?;
        Ok(PyOperatorRegistry {
            inner: registry,
        })
    }
    fn execute(&self, operator: String, params: String) -> PyResult<Value> {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { self.inner.execute(&operator, &params).await })
            .map_err(|e| PyRuntimeError::new_err(
                format!("Operator execution failed: {}", e),
            ))?;
        Ok(Value { inner: result })
    }
    fn context(&self) -> PyExecutionContext {
        use std::sync::Arc;
        let context = Arc::clone(&self.inner.context());
        PyExecutionContext {
            inner: (*context).clone(),
        }
    }
}
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
        self.data
            .get(&key)
            .map(|value| {
                Python::with_gil(|py| {
                    match value {
                        HlxValue::String(s) => s.clone().into_py(py),
                        HlxValue::Number(n) => n.into_py(py),
                        HlxValue::Bool(b) => b.into_py(py),
                        HlxValue::Array(_) => format!("{:?}", value).into_py(py),
                        HlxValue::Object(_) => format!("{:?}", value).into_py(py),
                        HlxValue::Null => py.None(),
                        HlxValue::Duration(d) => format!("{:?}", d).into_py(py),
                        HlxValue::Reference(s) => s.clone().into_py(py),
                        HlxValue::Identifier(s) => s.clone().into_py(py),
                    }
                })
            })
    }
    fn set(&mut self, key: String, value: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
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
                (
                    k.clone(),
                    Python::with_gil(|py| {
                        value_to_pyobject(v).unwrap_or_else(|_| py.None())
                    }),
                )
            })
            .collect()
    }
}
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
            .map_err(|e| PyRuntimeError::new_err(
                format!("Failed to create interpreter: {}", e),
            ))?;
        Ok(PyHelixInterpreter {
            inner: interpreter,
        })
    }
    fn execute<'py>(&self, py: Python<'py>, expression: String) -> PyResult<PyObject> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(
                format!("Failed to create runtime: {}", e),
            ))?;
        rt.block_on(async {
            let mut interpreter = HelixInterpreter::new()
                .await
                .map_err(|e| PyRuntimeError::new_err(
                    format!("Failed to create interpreter: {}", e),
                ))?;
            if let Some(value) = interpreter.get_variable(&expression) {
                types_value_to_pyobject(&value)
            } else {
                Ok(format!("Executed: {}", expression).into_py(py))
            }
        })
    }
    fn set_variable(&mut self, name: String, value: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            let helix_value = match value.extract::<String>(py) {
                Ok(s) => HlxValue::String(s),
                Err(_) => match value.extract::<f64>(py) {
                    Ok(n) => HlxValue::Number(n),
                    Err(_) => match value.extract::<bool>(py) {
                        Ok(b) => HlxValue::Bool(b),
                        Err(_) => HlxValue::String(format!("{:?}", value.bind(py))),
                    },
                },
            };
            // Convert from atp::value::Value to atp::types::Value
            let types_value = match helix_value {
                HlxValue::String(s) => crate::dna::atp::types::Value::String(s),
                HlxValue::Number(n) => crate::dna::atp::types::Value::Number(n),
                HlxValue::Bool(b) => crate::dna::atp::types::Value::Bool(b),
                HlxValue::Array(arr) => crate::dna::atp::types::Value::Array(
                    arr.into_iter().map(|v| match v {
                        HlxValue::String(s) => crate::dna::atp::types::Value::String(s),
                        HlxValue::Number(n) => crate::dna::atp::types::Value::Number(n),
                        HlxValue::Bool(b) => crate::dna::atp::types::Value::Bool(b),
                        HlxValue::Null => crate::dna::atp::types::Value::Null,
                        _ => crate::dna::atp::types::Value::String(format!("{:?}", v)),
                    }).collect()
                ),
                HlxValue::Object(obj) => crate::dna::atp::types::Value::Object(
                    obj.into_iter().map(|(k, v)| (k, match v {
                        HlxValue::String(s) => crate::dna::atp::types::Value::String(s),
                        HlxValue::Number(n) => crate::dna::atp::types::Value::Number(n),
                        HlxValue::Bool(b) => crate::dna::atp::types::Value::Bool(b),
                        HlxValue::Null => crate::dna::atp::types::Value::Null,
                        _ => crate::dna::atp::types::Value::String(format!("{:?}", v)),
                    })).collect()
                ),
                HlxValue::Null => crate::dna::atp::types::Value::Null,
                HlxValue::Duration(d) => crate::dna::atp::types::Value::Duration(d),
                HlxValue::Reference(r) => crate::dna::atp::types::Value::Reference(r),
                HlxValue::Identifier(i) => crate::dna::atp::types::Value::Identifier(i),
            };
            self.inner.set_variable(name, types_value);
            Ok(())
        })
    }
    fn get_variable(&self, name: String) -> Option<PyObject> {
        if let Some(value) = self.inner.get_variable(&name) {
            Python::with_gil(|py| {
                let obj: PyObject = match value {
                    crate::dna::atp::types::Value::String(s) => s.clone().into_py(py),
                    crate::dna::atp::types::Value::Number(n) => (*n).into_py(py),
                    crate::dna::atp::types::Value::Bool(b) => (*b).into_py(py),
                    _ => format!("{:?}", value).into_py(py),
                };
                Some(obj)
            })
        } else {
            None
        }
    }
}
#[pyfunction]
fn parse(source: String) -> PyResult<PyHelixConfig> {
    let config_result = crate::parse_and_validate(&source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {}", e)))?;
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
                                    serde_json::Value::String(s) => {
                                        helix_array.push(HlxValue::String(s))
                                    }
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_array.push(HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => {
                                        helix_array.push(HlxValue::Bool(b))
                                    }
                                    _ => helix_array.push(HlxValue::String(format!("{}", item))),
                                }
                            }
                            HlxValue::Array(helix_array)
                        }
                        serde_json::Value::Object(obj) => {
                            let mut helix_obj = HashMap::new();
                            for (obj_k, obj_v) in obj {
                                match obj_v {
                                    serde_json::Value::String(s) => {
                                        helix_obj.insert(obj_k, HlxValue::String(s));
                                    }
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_obj.insert(obj_k, HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => {
                                        helix_obj.insert(obj_k, HlxValue::Bool(b));
                                    }
                                    _ => {
                                        helix_obj
                                            .insert(obj_k, HlxValue::String(format!("{}", obj_v)));
                                    }
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
fn execute(
    expression: String,
    context: Option<HashMap<String, PyObject>>,
) -> PyResult<PyObject> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(
            format!("Failed to create runtime: {}", e),
        ))?;
    rt.block_on(async {
        let mut interpreter = HelixInterpreter::new()
            .await
            .map_err(|e| PyRuntimeError::new_err(
                format!("Failed to create interpreter: {}", e),
            ))?;
        if let Some(ctx) = context {
            for (key, value) in ctx {
                let helix_value = Python::with_gil(|py| {
                    match value.extract::<String>(py) {
                        Ok(s) => HlxValue::String(s),
                        Err(_) => match value.extract::<f64>(py) {
                            Ok(n) => HlxValue::Number(n),
                            Err(_) => match value.extract::<bool>(py) {
                                Ok(b) => HlxValue::Bool(b),
                                Err(_) => HlxValue::String(format!("{:?}", value.bind(py))),
                            },
                        },
                    }
                });
                // Convert from atp::value::Value to atp::types::Value
                let types_value = match helix_value {
                    HlxValue::String(s) => crate::dna::atp::types::Value::String(s),
                    HlxValue::Number(n) => crate::dna::atp::types::Value::Number(n),
                    HlxValue::Bool(b) => crate::dna::atp::types::Value::Bool(b),
                    HlxValue::Array(arr) => crate::dna::atp::types::Value::Array(
                        arr.into_iter().map(|v| match v {
                            HlxValue::String(s) => crate::dna::atp::types::Value::String(s),
                            HlxValue::Number(n) => crate::dna::atp::types::Value::Number(n),
                            HlxValue::Bool(b) => crate::dna::atp::types::Value::Bool(b),
                            HlxValue::Null => crate::dna::atp::types::Value::Null,
                            _ => crate::dna::atp::types::Value::String(format!("{:?}", v)),
                        }).collect()
                    ),
                    HlxValue::Object(obj) => crate::dna::atp::types::Value::Object(
                        obj.into_iter().map(|(k, v)| (k, match v {
                            HlxValue::String(s) => crate::dna::atp::types::Value::String(s),
                            HlxValue::Number(n) => crate::dna::atp::types::Value::Number(n),
                            HlxValue::Bool(b) => crate::dna::atp::types::Value::Bool(b),
                            HlxValue::Null => crate::dna::atp::types::Value::Null,
                            _ => crate::dna::atp::types::Value::String(format!("{:?}", v)),
                        })).collect()
                    ),
                    HlxValue::Null => crate::dna::atp::types::Value::Null,
                    HlxValue::Duration(d) => crate::dna::atp::types::Value::Duration(d),
                    HlxValue::Reference(r) => crate::dna::atp::types::Value::Reference(r),
                    HlxValue::Identifier(i) => crate::dna::atp::types::Value::Identifier(i),
                };
                interpreter.set_variable(key.to_string(), types_value);
            }
        }
        if let Some(value) = interpreter.get_variable(&expression) {
            Python::with_gil(|py| {
                let obj: PyObject = match value {
                    crate::dna::atp::types::Value::String(s) => s.clone().into_py(py),
                    crate::dna::atp::types::Value::Number(n) => (*n).into_py(py),
                    crate::dna::atp::types::Value::Bool(b) => (*b).into_py(py),
                    _ => format!("{:?}", value).into_py(py),
                };
                Ok(obj)
            })
        } else {
            Python::with_gil(|py| {
                Ok(format!("Executed: {}", expression).into_py(py))
            })
        }
    })
}
#[pyfunction]
fn load_file(file_path: String) -> PyResult<PyHelixConfig> {
    let config_result = crate::load_file(&file_path)
        .map_err(|e| PyValueError::new_err(format!("File load error: {}", e)))?;
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
                                    serde_json::Value::String(s) => {
                                        helix_array.push(HlxValue::String(s))
                                    }
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_array.push(HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => {
                                        helix_array.push(HlxValue::Bool(b))
                                    }
                                    _ => helix_array.push(HlxValue::String(format!("{}", item))),
                                }
                            }
                            HlxValue::Array(helix_array)
                        }
                        serde_json::Value::Object(obj) => {
                            let mut helix_obj = HashMap::new();
                            for (obj_k, obj_v) in obj {
                                match obj_v {
                                    serde_json::Value::String(s) => {
                                        helix_obj.insert(obj_k, HlxValue::String(s));
                                    }
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            helix_obj.insert(obj_k, HlxValue::Number(f));
                                        }
                                    }
                                    serde_json::Value::Bool(b) => {
                                        helix_obj.insert(obj_k, HlxValue::Bool(b));
                                    }
                                    _ => {
                                        helix_obj
                                            .insert(obj_k, HlxValue::String(format!("{}", obj_v)));
                                    }
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
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Value>()?;
    m.add_class::<PyExecutionContext>()?;
    m.add_class::<PyOperatorRegistry>()?;
    m.add_class::<PyHelixConfig>()?;
    m.add_class::<PyHelixInterpreter>()?;
    m.add_function(wrap_pyfunction_bound!(parse, m)?)?;
    m.add_function(wrap_pyfunction_bound!(execute, m)?)?;
    m.add_function(wrap_pyfunction_bound!(load_file, m)?)?;
    Ok(())
}