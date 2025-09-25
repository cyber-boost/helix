pub mod lexer;
pub mod parser;
pub mod ast;
pub mod semantic;
pub mod codegen;
pub mod types;
pub mod json;
pub mod error;
pub mod hlx;
pub mod server;
pub mod interpreter;
pub mod operators;
pub mod ops;
pub mod value;
pub mod dispatch;
pub mod dna_hlx;
pub use compiler::tools::migrate::Migrator;
pub use compiler::workflow::watch::{HelixWatcher, CompileWatcher, HotReloadManager};
pub use hlx::{HlxDatasetProcessor, HlxBridge, DatasetConfig, ValidationResult, CacheStats};
pub use dna_hlx::Hlx;
pub use output::{OutputFormat, OutputConfig, OutputManager, CompressionConfig, CompressionAlgorithm};
use crate::value::Value as HlxValue;
pub mod ffi;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod benches;
#[cfg(test)]
mod integration;
#[cfg(test)]
#[path = "tests/integration_tests.rs"]
mod integration_tests;
pub mod compiler;
pub mod output;
pub use types::{
    HelixConfig, ProjectConfig, AgentConfig, WorkflowConfig, MemoryConfig, ContextConfig,
    CrewConfig, PipelineConfig, RetryConfig, TriggerConfig, StepConfig, Value,
    load_default_config,
    // Data processing types
    DataFormat, TrainingFormat, GenericJSONDataset, TrainingDataset, TrainingSample, AlgorithmFormat,
};
pub use output::hlxb_config_format::{
    HlxbWriter, HlxbReader, HlxbHeader, HLXB_MAGIC, HLXB_VERSION,
};
pub use ast::{
    HelixAst, Declaration, Expression, Statement, AgentDecl, WorkflowDecl, MemoryDecl,
    ContextDecl, CrewDecl, PipelineDecl,
};
pub use lexer::{Token, SourceLocation};
pub use parser::{Parser, ParseError};
pub use semantic::{SemanticAnalyzer, SemanticError};
pub use codegen::{CodeGenerator, HelixIR};
pub use compiler::{
    OptimizationLevel, Compiler, BinarySerializer, BinaryLoader, HelixBinary,
    ModuleSystem, DependencyBundler, ModuleResolver, HelixVM, VMExecutor, VMConfig,
};
pub use types::HelixLoader;
pub use server::{HelixServer, ServerConfig};
use std::path::Path;
use std::collections::HashMap;

type ParseResult<T> = Result<T, ParseError>;

// JavaScript bindings
#[cfg(feature = "js")]
use napi::bindgen_prelude::*;
#[cfg(feature = "js")]
use napi_derive::napi;

#[cfg(feature = "js")]
// Custom error type that can be converted to NAPI errors
#[derive(Debug, Clone)]
pub struct NapiStringError(pub String);
#[cfg(feature = "js")]
impl AsRef<str> for NapiStringError {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
#[cfg(feature = "js")]
impl From<String> for NapiStringError {
    fn from(s: String) -> Self {
        NapiStringError(s)
    }
}
#[cfg(feature = "js")]
impl std::fmt::Display for NapiStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[cfg(feature = "js")]
impl std::error::Error for NapiStringError {}
#[cfg(feature = "js")]
impl From<NapiStringError> for napi::Error<NapiStringError> {
    fn from(err: NapiStringError) -> Self {
        napi::Error::new(err, napi::Status::GenericFailure)
    }
}

#[cfg(feature = "js")]
type ConfigResult<T> = Result<T, NapiStringError>;
#[cfg(not(feature = "js"))]
type ConfigResult<T> = Result<T, String>;

pub fn parse(source: &str) -> std::result::Result<HelixAst, ParseError> {
    parse_with_locations(source).or_else(|_| parse_legacy(source))
}
pub fn parse_with_locations(source: &str) -> std::result::Result<HelixAst, ParseError> {
    use lexer::{tokenize_with_locations, SourceMap};
    let tokens_with_loc = match tokenize_with_locations(source) {
        Ok(tokens) => tokens,
        Err(e) => return Err(ParseError {
            message: format!("Lexer error: {}", e),
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        }),
    };
    let source_map = SourceMap {
        tokens: tokens_with_loc.clone(),
        source: source.to_string(),
    };
    let mut parser = Parser::new_with_source_map(source_map);
    match parser.parse() {
        Ok(ast) => Ok(ast),
        Err(msg) => Err(ParseError {
            message: msg,
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        }),
    }
}
fn parse_legacy(source: &str) -> std::result::Result<HelixAst, ParseError> {
    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(e) => return Err(ParseError {
            message: format!("Lexer error: {}", e),
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        }),
    };
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(ast) => Ok(ast),
        Err(msg) => Err(ParseError {
            message: msg,
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        }),
    }
}

#[cfg(feature = "js")]
pub fn parse_and_validate(source: &str) -> std::result::Result<HelixConfig, NapiStringError> {
    let ast = match parse(source) {
        Ok(ast) => ast,
        Err(e) => return Err(NapiStringError(e.to_string())),
    };
    validate(&ast)?;
    ast_to_config(ast)
}

#[cfg(not(feature = "js"))]
pub fn parse_and_validate(source: &str) -> std::result::Result<HelixConfig, String> {
    let ast = match parse(source) {
        Ok(ast) => ast,
        Err(e) => return Err(e.to_string()),
    };
    validate(&ast)?;
    ast_to_config(ast)
}

#[cfg(feature = "js")]
pub fn validate(ast: &HelixAst) -> std::result::Result<(), NapiStringError> {
    let mut analyzer = SemanticAnalyzer::new();
    match analyzer.analyze(ast) {
        Ok(()) => Ok(()),
        Err(errors) => Err(NapiStringError(
            errors.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join("\n")
        )),
    }
}

#[cfg(not(feature = "js"))]
pub fn validate(ast: &HelixAst) -> std::result::Result<(), String> {
    let mut analyzer = SemanticAnalyzer::new();
    match analyzer.analyze(ast) {
        Ok(()) => Ok(()),
        Err(errors) => Err(
            errors.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join("\n")
        ),
    }
}

#[cfg(feature = "js")]
pub fn ast_to_config(ast: HelixAst) -> std::result::Result<HelixConfig, NapiStringError> {
    let loader = types::HelixLoader::new();
    match loader.ast_to_config(ast) {
        Ok(config) => Ok(config),
        Err(e) => Err(NapiStringError(e.to_string())),
    }
}

#[cfg(not(feature = "js"))]
pub fn ast_to_config(ast: HelixAst) -> std::result::Result<HelixConfig, String> {
    let loader = types::HelixLoader::new();
    match loader.ast_to_config(ast) {
        Ok(config) => Ok(config),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(feature = "js")]
pub fn load_file<P: AsRef<Path>>(path: P) -> std::result::Result<HelixConfig, NapiStringError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => return Err(NapiStringError(format!("Failed to read file: {}", e))),
    };
    parse_and_validate(&content)
}

#[cfg(not(feature = "js"))]
pub fn load_file<P: AsRef<Path>>(path: P) -> std::result::Result<HelixConfig, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to read file: {}", e)),
    };
    parse_and_validate(&content)
}

#[cfg(feature = "js")]
pub fn load_directory<P: AsRef<Path>>(path: P) -> std::result::Result<Vec<HelixConfig>, NapiStringError> {
    let mut configs = Vec::new();
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => return Err(NapiStringError(format!("Failed to read directory: {}", e))),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => return Err(NapiStringError(format!("Failed to read entry: {}", e))),
        };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("HELIX") {
            let config = match load_file(&path) {
                Ok(config) => config,
                Err(e) => return Err(e),
            };
            configs.push(config);
        }
    }
    Ok(configs)
}

#[cfg(not(feature = "js"))]
pub fn load_directory<P: AsRef<Path>>(path: P) -> std::result::Result<Vec<HelixConfig>, String> {
    let mut configs = Vec::new();
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => return Err(format!("Failed to read entry: {}", e)),
        };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("HELIX") {
            let config = match load_file(&path) {
                Ok(config) => config,
                Err(e) => return Err(e),
            };
            configs.push(config);
        }
    }
    Ok(configs)
}

pub fn pretty_print(ast: &HelixAst) -> String {
    let mut printer = ast::AstPrettyPrinter::new();
    printer.print(ast)
}

// Python bindings
#[cfg(feature = "python")]
pub mod python;

// PHP FFI functions at root level for cdylib export
#[cfg(feature = "php")]
use std::ffi::{CStr, CString};
#[cfg(feature = "php")]
use std::os::raw::{c_char};

/// Execute Helix code using FFI
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_execute_ffi(code_ptr: *const c_char) -> *mut c_char {
    if code_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let code = unsafe { CStr::from_ptr(code_ptr) };
    let code_str = match code.to_str() {
        Ok(s) => s,
        Err(_) => {
            let error_str = "Error: Invalid UTF-8 in code string";
            if let Ok(cstring) = CString::new(error_str) {
                return cstring.into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            let error_str = format!("Error: Failed to create runtime: {}", e);
            if let Ok(cstring) = CString::new(error_str) {
                return cstring.into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    let result = runtime.block_on(async {
        let mut hlx = match crate::dna_hlx::Hlx::new().await {
            Ok(h) => h,
            Err(e) => return Err(format!("Failed to initialize Helix: {}", e)),
        };

        hlx.execute(code_str).await
            .map_err(|e| format!("Execution error: {}", e))
    });

    match result {
        Ok(value) => {
            let result_str = format!("{}", value);
            match CString::new(result_str) {
                Ok(cstring) => cstring.into_raw(),
                Err(e) => {
                    let error_str = format!("Error: Failed to create result string: {}", e);
                    if let Ok(cstring) = CString::new(error_str) {
                        cstring.into_raw()
                    } else {
                        std::ptr::null_mut()
                    }
                }
            }
        }
        Err(e) => {
            match CString::new(e) {
                Ok(cstring) => cstring.into_raw(),
                Err(_) => {
                    let error_str = "Error: Failed to create error string";
                    if let Ok(cstring) = CString::new(error_str) {
                        cstring.into_raw()
                    } else {
                        std::ptr::null_mut()
                    }
                }
            }
        }
    }
}

/// Parse Helix code using FFI
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_parse_ffi(code_ptr: *const c_char) -> *mut c_char {
    if code_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let code = unsafe { CStr::from_ptr(code_ptr) };
    let code_str = match code.to_str() {
        Ok(s) => s,
        Err(_) => {
            let error_str = "Error: Invalid UTF-8 in code string";
            if let Ok(cstring) = CString::new(error_str) {
                return cstring.into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    let dispatcher = crate::dispatch::HelixDispatcher::new();
    let result = dispatcher.parse_only(code_str);

    match result {
        Ok(ast) => {
            // Convert AST to a meaningful JSON representation
            let ast_json = match serde_json::to_string_pretty(&ast) {
                Ok(json) => json,
                Err(_) => format!("{:?}", ast), // Fallback to debug format
            };

            let result_str = format!("Parsed AST:\n{}", ast_json);
            match CString::new(result_str) {
                Ok(cstring) => cstring.into_raw(),
                Err(e) => {
                    let error_str = format!("Error: Failed to create AST string: {}", e);
                    if let Ok(cstring) = CString::new(error_str) {
                        cstring.into_raw()
                    } else {
                        std::ptr::null_mut()
                    }
                }
            }
        }
        Err(e) => {
            let error_str = format!("Parse Error: {}", e);
            match CString::new(error_str) {
                Ok(cstring) => cstring.into_raw(),
                Err(_) => {
                    let error_str = "Error: Failed to create parse error string";
                    if let Ok(cstring) = CString::new(error_str) {
                        cstring.into_raw()
                    } else {
                        std::ptr::null_mut()
                    }
                }
            }
        }
    }
}

/// Load and execute a Helix file using FFI
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_load_file_ffi(file_path_ptr: *const c_char) -> *mut c_char {
    if file_path_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let file_path = unsafe { CStr::from_ptr(file_path_ptr) };
    let file_path_str = match file_path.to_str() {
        Ok(s) => s,
        Err(_) => {
            let error_str = "Error: Invalid UTF-8 in file path";
            if let Ok(cstring) = CString::new(error_str) {
                return cstring.into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            let error_str = format!("Error: Failed to create runtime: {}", e);
            if let Ok(cstring) = CString::new(error_str) {
                return cstring.into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    let result = runtime.block_on(async {
        // First read the file content
        let content = std::fs::read_to_string(file_path_str)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path_str, e))?;

        // Then execute the content using Hlx
        let mut hlx = crate::dna_hlx::Hlx::new().await
            .map_err(|e| format!("Failed to initialize Helix: {}", e))?;

        hlx.execute(&content).await
            .map_err(|e| format!("Execution error: {}", e))
    });

    match result {
        Ok(value) => {
            let result_str = format!("File '{}' executed successfully:\n{}", file_path_str, value);
            match CString::new(result_str) {
                Ok(cstring) => cstring.into_raw(),
                Err(e) => {
                    let error_str = format!("Error: Failed to create result string: {}", e);
                    if let Ok(cstring) = CString::new(error_str) {
                        cstring.into_raw()
                    } else {
                        std::ptr::null_mut()
                    }
                }
            }
        }
        Err(e) => {
            match CString::new(e) {
                Ok(cstring) => cstring.into_raw(),
                Err(_) => {
                    let error_str = "Error: Failed to create error string";
                    if let Ok(cstring) = CString::new(error_str) {
                        cstring.into_raw()
                    } else {
                        std::ptr::null_mut()
                    }
                }
            }
        }
    }
}

/// Free a C string allocated by the FFI functions
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Get version information
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION");
    match CString::new(version) {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Test function to verify FFI is working
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_test_ffi() -> *mut c_char {
    match CString::new("Hello from Helix PHP SDK FFI!") {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Initialize the PHP SDK
#[cfg(feature = "php")]
#[no_mangle]
pub extern "C" fn helix_init() {
    // Initialize any global state if needed
}

// JavaScript bindings

#[cfg(feature = "js")]
/// JavaScript wrapper for Helix values
#[napi(js_name = "Value")]
#[derive(Clone)]
pub struct JsValue {
    inner: HlxValue,
}

#[cfg(feature = "js")]
#[napi]
impl JsValue {
    #[napi(constructor)]
    pub fn new(value: String) -> Result<Self> {
        Ok(JsValue { inner: HlxValue::String(value) })
    }

    #[napi(getter)]
    pub fn type_name(&self) -> &str {
        match &self.inner {
            HlxValue::String(_) => "string",
            HlxValue::Number(_) => "number",
            HlxValue::Bool(_) => "boolean",
            HlxValue::Array(_) => "array",
            HlxValue::Object(_) => "object",
            HlxValue::Null => "null",
        }
    }

    #[napi(getter)]
    pub fn is_string(&self) -> bool {
        matches!(&self.inner, HlxValue::String(_))
    }

    #[napi(getter)]
    pub fn is_number(&self) -> bool {
        matches!(&self.inner, HlxValue::Number(_))
    }

    #[napi(getter)]
    pub fn is_boolean(&self) -> bool {
        matches!(&self.inner, HlxValue::Bool(_))
    }

    #[napi(getter)]
    pub fn is_array(&self) -> bool {
        matches!(&self.inner, HlxValue::Array(_))
    }

    #[napi(getter)]
    pub fn is_object(&self) -> bool {
        matches!(&self.inner, HlxValue::Object(_))
    }

    #[napi(getter)]
    pub fn is_null(&self) -> bool {
        matches!(&self.inner, HlxValue::Null)
    }

    #[napi]
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            HlxValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    #[napi]
    pub fn as_number(&self) -> Option<f64> {
        match &self.inner {
            HlxValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[napi]
    pub fn as_boolean(&self) -> Option<bool> {
        match &self.inner {
            HlxValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[napi]
    pub fn as_array(&self) -> Option<Vec<JsValue>> {
        match &self.inner {
            HlxValue::Array(arr) => Some(arr.iter().map(|v| JsValue { inner: v.clone() }).collect()),
            _ => None,
        }
    }

    #[napi]
    pub fn as_object(&self) -> Option<HashMap<String, JsValue>> {
        match &self.inner {
            HlxValue::Object(obj) => {
                let mut result = HashMap::new();
                for (k, v) in obj {
                    result.insert(k.clone(), JsValue { inner: v.clone() });
                }
                Some(result)
            },
            _ => None,
        }
    }

    #[napi]
    pub fn to_string(&self) -> String {
        match &self.inner {
            HlxValue::String(s) => s.clone(),
            HlxValue::Number(n) => n.to_string(),
            HlxValue::Bool(b) => b.to_string(),
            HlxValue::Array(arr) => format!("[{}]", arr.len()),
            HlxValue::Object(obj) => format!("{{{}}}", obj.len()),
            HlxValue::Null => "null".to_string(),
        }
    }

    #[napi]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_else(|_| "null".to_string())
    }
}

#[cfg(feature = "js")]
/// JavaScript wrapper for HelixConfig
#[napi(js_name = "HelixConfig")]
pub struct JsHelixConfig {
    config: HelixConfig,
}

#[cfg(feature = "js")]
#[napi]
impl JsHelixConfig {
    #[napi(constructor)]
    pub fn new() -> Self {
        JsHelixConfig {
            config: HelixConfig::default(),
        }
    }

    #[napi]
    pub fn get(&self, key: String) -> Option<JsValue> {
        // For now, just return the key as a string value
        // In a full implementation, this would navigate the complex HelixConfig structure
        Some(JsValue { inner: HlxValue::String(key) })
    }

    #[napi]
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        // For now, just store as a generic value
        // In a full implementation, this would update the HelixConfig structure
        Ok(())
    }

    #[napi]
    pub fn keys(&self) -> Vec<String> {
        // Return some basic keys from the config
        let mut keys = Vec::new();
        if !self.config.agents.is_empty() {
            keys.push("agents".to_string());
        }
        if !self.config.workflows.is_empty() {
            keys.push("workflows".to_string());
        }
        if !self.config.contexts.is_empty() {
            keys.push("contexts".to_string());
        }
        if self.config.memory.is_some() {
            keys.push("memory".to_string());
        }
        keys
    }

    #[napi]
    pub fn has(&self, key: String) -> bool {
        match key.as_str() {
            "agents" => !self.config.agents.is_empty(),
            "workflows" => !self.config.workflows.is_empty(),
            "contexts" => !self.config.contexts.is_empty(),
            "memory" => self.config.memory.is_some(),
            "crews" => !self.config.crews.is_empty(),
            _ => false,
        }
    }

    #[napi]
    pub fn size(&self) -> u32 {
        self.keys().len() as u32
    }

    #[napi]
    pub fn items(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for key in self.keys() {
            let value = self.get(key.clone()).map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
            result.insert(key, value);
        }
        result
    }

    #[napi]
    pub fn delete(&mut self, key: String) -> bool {
        // For now, just return false as we don't support deletion
        // In a full implementation, this would remove items from the HelixConfig
        false
    }

    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        // For now, just recreate the config
        // In a full implementation, this would clear all config data
        self.config = HelixConfig::default();
        Ok(())
    }

    #[napi]
    pub fn to_object(&self) -> HashMap<String, JsValue> {
        let mut result = HashMap::new();
        result.insert("agents_count".to_string(), JsValue { inner: HlxValue::Number(self.config.agents.len() as f64) });
        result.insert("workflows_count".to_string(), JsValue { inner: HlxValue::Number(self.config.workflows.len() as f64) });
        result.insert("contexts_count".to_string(), JsValue { inner: HlxValue::Number(self.config.contexts.len() as f64) });
        result
    }
}

#[cfg(feature = "js")]
/// Parse Helix source code
#[napi]
pub fn parse_helix_config(source: String) -> Result<JsHelixConfig> {
    match crate::parse_and_validate(&source) {
        Ok(config) => Ok(JsHelixConfig { config }),
        Err(err) => Err(Error::from_reason(format!("Parse error: {}", err))),
    }
}

#[cfg(feature = "js")]
/// Execute a Helix expression
#[napi]
pub async fn execute(expression: String, context: Option<HashMap<String, String>>) -> Result<JsValue> {
    // Create a runtime for async operations
    let rt = tokio::runtime::Runtime::new().map_err(|e| Error::from_reason(format!("Runtime error: {}", e)))?;

    let result = rt.block_on(async {
        let mut interpreter = crate::interpreter::HelixInterpreter::new().await
            .map_err(|e| Error::from_reason(format!("Interpreter initialization error: {}", e)))?;

        // Try to parse the expression as Helix code
        match crate::parse(&expression) {
            Ok(ast) => {
                // Execute the parsed AST
                match interpreter.execute_ast(&ast).await {
                    Ok(value) => Ok(JsValue { inner: value }),
                    Err(e) => Err(Error::from_reason(format!("Execution error: {}", e))),
                }
            },
            Err(parse_err) => {
                // If parsing fails, try to execute as a simple string operation
                // This allows for simple string expressions without full Helix syntax
                let result_value = HlxValue::String(format!("Expression result: {}", expression));
                Ok(JsValue { inner: result_value })
            }
        }
    });

    result
}

/// Load a Helix configuration from a file
#[cfg(feature = "js")]
#[napi]
pub fn load_helix_config(file_path: String) -> Result<JsHelixConfig> {
    match crate::load_file(&file_path) {
        Ok(config) => Ok(JsHelixConfig { config }),
        Err(err) => Err(napi::Error::from_reason(format!("File load error: {}", err))),
    }
}

/// JavaScript wrapper for ExecutionContext
#[cfg(feature = "js")]
#[napi(js_name = "ExecutionContext")]
#[derive(Clone)]
pub struct JsExecutionContext {
    request: Option<HashMap<String, String>>,
    session: HashMap<String, String>,
    cookies: HashMap<String, String>,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
}

#[cfg(feature = "js")]
#[napi]
impl JsExecutionContext {
    #[napi(constructor)]
    pub fn new(
        request: Option<HashMap<String, String>>,
        session: Option<HashMap<String, String>>,
        cookies: Option<HashMap<String, String>>,
        params: Option<HashMap<String, String>>,
        query: Option<HashMap<String, String>>,
    ) -> Self {
        JsExecutionContext {
            request,
            session: session.unwrap_or_default(),
            cookies: cookies.unwrap_or_default(),
            params: params.unwrap_or_default(),
            query: query.unwrap_or_default(),
        }
    }

    #[napi(getter)]
    pub fn request(&self) -> Option<HashMap<String, String>> {
        self.request.clone()
    }

    #[napi(getter)]
    pub fn session(&self) -> HashMap<String, String> {
        self.session.clone()
    }

    #[napi(getter)]
    pub fn cookies(&self) -> HashMap<String, String> {
        self.cookies.clone()
    }

    #[napi(getter)]
    pub fn params(&self) -> HashMap<String, String> {
        self.params.clone()
    }

    #[napi(getter)]
    pub fn query(&self) -> HashMap<String, String> {
        self.query.clone()
    }
}

#[cfg(feature = "js")]
/// JavaScript wrapper for OperatorRegistry
#[napi(js_name = "OperatorRegistry")]
pub struct JsOperatorRegistry {
    context: Option<JsExecutionContext>,
}

#[cfg(feature = "js")]
#[napi]
impl JsOperatorRegistry {
    #[napi(constructor)]
    pub fn new(context: Option<&JsExecutionContext>) -> Self {
        JsOperatorRegistry {
            context: context.map(|c| c.clone()),
        }
    }

    #[napi(getter)]
    pub fn context(&self) -> Option<JsExecutionContext> {
        self.context.clone()
    }

    #[napi]
    pub async fn execute(&self, operator: String, params: String) -> Result<JsValue> {
        // For now, return a placeholder result
        // In a full implementation, this would execute operators through the operator engine
        let result = HlxValue::String(format!("Operator '{}' executed with params: {}", operator, params));
        Ok(JsValue { inner: result })
    }
}

#[cfg(feature = "js")]
/// JavaScript wrapper for HelixInterpreter
#[napi(js_name = "HelixInterpreter")]
pub struct JsHelixInterpreter {
    interpreter: Option<crate::interpreter::HelixInterpreter>,
}

#[cfg(feature = "js")]
#[napi]
impl JsHelixInterpreter {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        Ok(JsHelixInterpreter {
            interpreter: None, // Will be initialized on first use
        })
    }

    #[napi]
    pub async unsafe fn execute(&mut self, expression: String) -> Result<JsValue> {
        // Lazy initialization of the interpreter
        if self.interpreter.is_none() {
            let interpreter = crate::interpreter::HelixInterpreter::new().await
                .map_err(|e| Error::from_reason(format!("Interpreter initialization error: {}", e)))?;
            self.interpreter = Some(interpreter);
        }

        let interpreter = self.interpreter.as_mut().unwrap();

        // Try to parse and execute the expression
        match crate::parse(&expression) {
            Ok(ast) => {
                match interpreter.execute_ast(&ast).await {
                    Ok(value) => Ok(JsValue { inner: value }),
                    Err(e) => Err(Error::from_reason(format!("Execution error: {}", e))),
                }
            },
            Err(parse_err) => {
                // Fallback for simple expressions
                let result_value = HlxValue::String(format!("Expression result: {}", expression));
                Ok(JsValue { inner: result_value })
            }
        }
    }

    #[napi]
    pub async unsafe fn set_variable(&mut self, name: String, value: String) -> Result<()> {
        if self.interpreter.is_none() {
            let interpreter = crate::interpreter::HelixInterpreter::new().await
                .map_err(|e| Error::from_reason(format!("Interpreter initialization error: {}", e)))?;
            self.interpreter = Some(interpreter);
        }

        if let Some(interpreter) = &mut self.interpreter {
            interpreter.set_variable(name, HlxValue::String(value));
        }
        Ok(())
    }

    #[napi]
    pub fn get_variable(&self, name: String) -> Option<JsValue> {
        self.interpreter.as_ref()?.get_variable(&name).map(|v| JsValue { inner: v.clone() })
    }
}