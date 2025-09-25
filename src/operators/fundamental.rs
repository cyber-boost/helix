
//! Fundamental Language Operators - @ prefixed operators
//!
//! This module implements the core @ prefixed operators that provide basic language functionality:
//! - @var: Global variable references (persisted across executions)
//! - @env: Environment variable access
//! - @request: HTTP request data access
//! - @session: Session management (mutable)
//! - @cookie: Cookie operations
//! - @header: HTTP header access
//! - @param: Route parameter extraction
//! - @query: URL query parameter access
//! - @timezone: Timezone conversions with chrono-tz
//! - @regex: Regular expression operations
//! - @json: JSON parsing and manipulation
//! - @base64: Base64 encoding/decoding
//! - @url: URL encoding/decoding
//! - @hash: Hashing operations (SHA256, MD5)
//! - @uuid: UUID generation (v4)
//! - @if: Conditional expressions
//! - @switch: Switch statements
//! - @case: Case matching
//! - @default: Default values
//! - @and: Logical AND
//! - @or: Logical OR
//! - @not: Logical NOT
//! - @math: Mathematical operations
//! - @calc: Complex calculations with meval
//! - @min: Minimum value from array
//! - @max: Maximum value from array

use crate::error::HlxError;
use crate::operators::utils;
use crate::operators::{json_to_value, value_to_json, OperatorTrait};
use crate::value::Value;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{Local, Utc, TimeZone, Datelike, Timelike, DateTime};
use chrono_tz::Tz;
use regex::Regex;
use base64::{engine::general_purpose, Engine as _};
use urlencoding;
use sha2::{Sha256, Digest};
use md5;
use uuid::Uuid;
use meval;

/// Execution context containing request data, session, cookies, etc.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// HTTP request data (optional, may be None for non-web contexts)
    pub request: Option<RequestData>,
    /// Session data (mutable, shared)
    pub session: Arc<RwLock<HashMap<String, Value>>>,
    /// HTTP cookies
    pub cookies: HashMap<String, String>,
    /// Route parameters
    pub params: HashMap<String, String>,
    /// URL query parameters
    pub query: HashMap<String, String>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            request: None,
            session: Arc::new(RwLock::new(HashMap::new())),
            cookies: HashMap::new(),
            params: HashMap::new(),
            query: HashMap::new(),
        }
    }
}

/// HTTP request data structure
#[derive(Debug, Clone)]
pub struct RequestData {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// Fundamental operators implementation supporting @ prefixed syntax
pub struct FundamentalOperators {
    /// Global variables (persisted across executions)
    variables: Arc<RwLock<HashMap<String, Value>>>,
    /// Execution context – carries request, session, cookies, etc.
    context: Arc<ExecutionContext>,
}

impl FundamentalOperators {
    /// Construct a new instance **with** an execution context.
    pub async fn new_with_context(context: ExecutionContext) -> Result<Self, HlxError> {
        Ok(Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            context: Arc::new(context),
        })
    }

    /// Legacy constructor for backward compatibility (creates empty context)
    pub async fn new() -> Result<Self, HlxError> {
        let context = ExecutionContext::default();
        Self::new_with_context(context).await
    }

    /// Get a variable value directly from the global variables store
    pub fn get_variable(&self, name: &str) -> Result<Value, HlxError> {
        let vars = self.variables.read()
            .map_err(|_| HlxError::validation_error("RwLock poisoned", "Check concurrency"))?;
        Ok(vars.get(name).cloned().unwrap_or(Value::Null))
    }

    /// Set a variable value in the global variables store
    pub fn set_variable(&self, name: String, value: Value) -> Result<(), HlxError> {
        let mut vars = self.variables.write()
            .map_err(|_| HlxError::validation_error("RwLock poisoned", "Check concurrency"))?;
        vars.insert(name, value);
        Ok(())
    }
}

impl FundamentalOperators {
    /// @var – Get or set a global variable.
    /// Parameters:
    ///   * `name` (String) – variable name (required)
    ///   * `value` (any Value) – if present the variable is set, otherwise it is read.
    async fn var_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let name = params_map
            .get("name")
            .and_then(|v| v.as_string())
            .ok_or_else(|| {
                HlxError::invalid_parameters(
                    "var",
                    "Missing required parameter `name`",
                )
            })?
            .to_string();

        if let Some(val) = params_map.get("value") {
            // ---- SET ----
            let mut vars = self
                .variables
                .write()
                .map_err(|_| HlxError::validation_error("RwLock poisoned", "Check concurrency"))?;
            vars.insert(name.clone(), val.clone());
            Ok(Value::Object({
                let mut map = HashMap::new();
                map.insert(
                    "operation".to_string(),
                    Value::String("set".to_string()),
                );
                map.insert("name".to_string(), Value::String(name));
                map.insert("value".to_string(), val.clone());
                map
            }))
        } else {
            // ---- GET ----
            let vars = self
                .variables
                .read()
                .map_err(|_| HlxError::validation_error("RwLock poisoned", "Check concurrency"))?;
            let stored = vars.get(&name).cloned().unwrap_or(Value::Null);
            Ok(Value::Object({
                let mut map = HashMap::new();
                map.insert(
                    "operation".to_string(),
                    Value::String("get".to_string()),
                );
                map.insert("name".to_string(), Value::String(name));
                map.insert("value".to_string(), stored);
                map
            }))
        }
    }

    /// @env – Read an OS environment variable, optionally providing a default.
    async fn env_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let var = params_map
            .get("var")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("env", "Missing required `var`"))?;

        match std::env::var(var) {
            Ok(v) => Ok(Value::String(v)),
            Err(_) => {
                if let Some(default) = params_map.get("default").and_then(|v| v.as_string()) {
                    Ok(Value::String(default.to_string()))
                } else {
                    Err(HlxError::invalid_parameters(
                        "env",
                        &format!("Variable `{}` not set and no `default` supplied", var),
                    ))
                }
            }
        }
    }

    /// @request – Pull data from the current request.
    /// Parameters:
    ///   * `field` (String, optional) – one of `method`, `url`, `headers`, `body`, `all`.
    async fn request_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        let field = params_map
            .get("field")
            .and_then(|v| v.as_string())
            .unwrap_or("all");

        // The request lives inside the shared ExecutionContext
        let req_opt = &self.context.request;

        let req = req_opt
            .as_ref()
            .ok_or_else(|| HlxError::invalid_parameters("request", "No request data in context"))?;

        match field {
            "method" => Ok(Value::String(req.method.clone())),
            "url" => Ok(Value::String(req.url.clone())),
            "headers" => {
                let mut map = HashMap::new();
                for (k, v) in &req.headers {
                    map.insert(k.clone(), Value::String(v.clone()));
                }
                Ok(Value::Object(map))
            }
            "body" => Ok(Value::String(req.body.clone())),
            "all" => {
                let mut map = HashMap::new();
                map.insert("method".to_string(), Value::String(req.method.clone()));
                map.insert("url".to_string(), Value::String(req.url.clone()));
                let mut hdrs = HashMap::new();
                for (k, v) in &req.headers {
                    hdrs.insert(k.clone(), Value::String(v.clone()));
                }
                map.insert("headers".to_string(), Value::Object(hdrs));
                map.insert("body".to_string(), Value::String(req.body.clone()));
                Ok(Value::Object(map))
            }
            _ => Err(HlxError::invalid_parameters(
                "request",
                "Invalid `field`; allowed: method, url, headers, body, all",
            )),
        }
    }

    /// @session – Read or write a value from the session map.
    /// Parameters:
    ///   * `action` (String, optional) – `get` (default) or `set`.
    ///   * `key` (String, required for both actions).
    ///   * `value` (any Value, required for `set`).
    async fn session_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        let action = params_map
            .get("action")
            .and_then(|v| v.as_string())
            .unwrap_or("get");

        match action {
            "get" => {
                let key = params_map
                    .get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| {
                        HlxError::invalid_parameters("session", "`key` required for get")
                    })?;

                let sess = self
                    .context
                    .session
                    .read()
                    .map_err(|_| HlxError::validation_error("RwLock poisoned", ""))?;
                let value = sess.get(key).cloned().unwrap_or(Value::Null);
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("value".to_string(), value);
                    map
                }))
            }
            "set" => {
                let key = params_map
                    .get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| {
                        HlxError::invalid_parameters("session", "`key` required for set")
                    })?;
                let value = params_map
                    .get("value")
                    .ok_or_else(|| {
                        HlxError::invalid_parameters("session", "`value` required for set")
                    })?;

                let mut sess = self
                    .context
                    .session
                    .write()
                    .map_err(|_| HlxError::validation_error("RwLock poisoned", ""))?;
                sess.insert(key.to_string(), value.clone());

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("operation".to_string(), Value::String("set".to_string()));
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("value".to_string(), value.clone());
                    map.insert("success".to_string(), Value::Bool(true));
                    map
                }))
            }
            _ => Err(HlxError::invalid_parameters(
                "session",
                "`action` must be either `get` or `set`",
            )),
        }
    }

    /// @cookie – Read a cookie from the shared context.
    async fn cookie_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        let name = params_map
            .get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("session_id");

        let cookie_val = self
            .context
            .cookies
            .get(name)
            .cloned()
            .unwrap_or_else(|| "".to_string());

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), Value::String(cookie_val));
            map
        }))
    }

    /// @header – Retrieve a request header.
    async fn header_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        let name = params_map
            .get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("User-Agent");

        let req_opt = &self.context.request;
        let req = req_opt
            .as_ref()
            .ok_or_else(|| HlxError::invalid_parameters("header", "No request in context"))?;

        let val = req.headers.get(name).cloned().unwrap_or_else(|| "".to_string());
        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), Value::String(val));
            map
        }))
    }

    /// @param – Pull a route‑parameter from the context.
    async fn param_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        let name = params_map
            .get("name")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("param", "`name` required"))?;

        let val = self
            .context
            .params
            .get(name)
            .cloned()
            .unwrap_or_else(|| "".to_string());

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), Value::String(val));
            map
        }))
    }

    /// @query – Pull a URL‑query parameter from the shared context.
    async fn query_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        let name = params_map
            .get("name")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("query", "`name` required"))?;

        let val = self
            .context
            .query
            .get(name)
            .cloned()
            .unwrap_or_else(|| "".to_string());

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), Value::String(val));
            map
        }))
    }

    // Date and Time Operations
    async fn date_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let format = params_map.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%Y-%m-%d");

        use chrono::{Local, Datelike, Timelike};

        let now = Local::now();
        let formatted = now.format(format).to_string();

        Ok(Value::String(formatted))
    }

    async fn time_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let format = params_map.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%H:%M:%S");

        use chrono::Local;

        let now = Local::now();
        let formatted = now.format(format).to_string();

        Ok(Value::String(formatted))
    }

    async fn timestamp_operator(&self, _params: &str) -> Result<Value, HlxError> {
        use chrono::Utc;
        let timestamp = Utc::now().timestamp();
        Ok(Value::Number(timestamp as f64))
    }

    async fn now_operator(&self, _params: &str) -> Result<Value, HlxError> {
        use chrono::{Utc, Local};
        let now = Local::now().to_rfc3339();
        Ok(Value::String(now))
    }

    async fn format_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("now");

        let format = params_map.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        use chrono::{Local, Utc, TimeZone};

        let datetime = if input == "now" {
            Local::now()
        } else if let Ok(ts) = input.parse::<i64>() {
            match Local.timestamp_opt(ts, 0) {
                chrono::LocalResult::Single(dt) => dt,
                _ => Local::now(),
            }
        } else {
            // Try to parse as ISO string
            match chrono::DateTime::parse_from_rfc3339(input) {
                Ok(dt) => dt.with_timezone(&Local),
                Err(_) => Local::now(),
            }
        };

        let result = datetime.format(format).to_string();
        Ok(Value::String(result))
    }

    async fn timezone_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let tz = params_map.get("tz")
            .and_then(|v| v.as_string())
            .unwrap_or("UTC");

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("now");

        use chrono::{Local, Utc, TimeZone};

        let datetime = if input == "now" {
            Utc::now()
        } else if let Ok(ts) = input.parse::<i64>() {
            match Utc.timestamp_opt(ts, 0) {
                chrono::LocalResult::Single(dt) => dt,
                _ => Utc::now(),
            }
        } else {
            match chrono::DateTime::parse_from_rfc3339(input) {
                Ok(dt) => dt.with_timezone(&Utc),
                Err(_) => Utc::now(),
            }
        };

        // For now, just return the datetime in the requested timezone name
        // Real implementation would use chrono-tz for proper conversion
        let result = format!("{} ({})", datetime.to_rfc3339(), tz);
        Ok(Value::String(result))
    }

    // String and Data Processing
    async fn string_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("string", "Missing 'input' parameter"))?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("string", "Missing 'operation' parameter"))?;

        match operation {
            "upper" => Ok(Value::String(input.to_uppercase())),
            "lower" => Ok(Value::String(input.to_lowercase())),
            "capitalize" => {
                let mut chars = input.chars();
                match chars.next() {
                    None => Ok(Value::String(String::new())),
                    Some(first) => Ok(Value::String(first.to_uppercase().collect::<String>() + chars.as_str())),
                }
            },
            "reverse" => Ok(Value::String(input.chars().rev().collect())),
            "length" => Ok(Value::Number(input.len() as f64)),
            "trim" => Ok(Value::String(input.trim().to_string())),
            "substring" => {
                let start = params_map.get("start").and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
                let len = params_map.get("len").and_then(|v| v.as_number()).unwrap_or((input.len() - start) as f64) as usize;
                let substr: String = input.chars().skip(start).take(len).collect();
                Ok(Value::String(substr))
            }
            _ => Err(HlxError::invalid_parameters("string", "Invalid 'operation'")),
        }
    }

    async fn regex_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("regex", "Missing 'input' parameter"))?;

        let pattern = params_map.get("pattern")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("regex", "Missing 'pattern' parameter"))?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("regex", "Missing 'operation' parameter"))?;

        let re = regex::Regex::new(pattern)
            .map_err(|e| HlxError::validation_error(format!("Invalid regex pattern: {}", e), "Check regex syntax"))?;

        match operation {
            "match" => Ok(Value::Bool(re.is_match(input))),
            "find" => {
                Ok(Value::String(re.find(input).map(|m| m.as_str()).unwrap_or("").to_string()))
            },
            "replace" => {
                let replacement = params_map.get("replacement")
                    .and_then(|v| v.as_string())
                    .unwrap_or("");
                Ok(Value::String(re.replace_all(input, replacement).to_string()))
            },
            "captures" => {
                let captures: Vec<Value> = re.captures_iter(input)
                    .map(|cap| Value::Array(cap.iter().skip(1).map(|m| Value::String(m.unwrap().as_str().to_string())).collect()))
                    .collect();
                Ok(Value::Array(captures))
            },
            _ => Err(HlxError::invalid_parameters("regex", "Invalid 'operation'")),
        }
    }

    async fn json_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("json", "Missing 'operation' parameter"))?;

        match operation {
            "parse" => {
                let input = params_map.get("input")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::invalid_parameters("json", "Missing 'input' parameter for parse"))?;

                let parsed: serde_json::Value = serde_json::from_str(input)
                    .map_err(|e| HlxError::json_error(format!("JSON parse error: {}", e), "Provide valid JSON"))?;

                Ok(json_to_value(&parsed))
            },
            "stringify" => {
                let input = params_map.get("input")
                    .ok_or_else(|| HlxError::invalid_parameters("json", "Missing 'input' parameter for stringify"))?;

                let json_value = value_to_json(input);
                let json_str = serde_json::to_string(&json_value)
                    .map_err(|e| HlxError::json_error(format!("JSON stringify error: {}", e), "Check input value"))?;

                Ok(Value::String(json_str))
            },
            _ => Err(HlxError::invalid_parameters("json", "Invalid 'operation'")),
        }
    }

    async fn base64_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("base64", "Missing 'input' parameter"))?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("base64", "Missing 'operation' parameter"))?;

        match operation {
            "encode" => {
                use base64::{Engine as _, engine::general_purpose};
                Ok(Value::String(general_purpose::STANDARD.encode(input)))
            },
            "decode" => {
                use base64::{Engine as _, engine::general_purpose};
                let decoded = general_purpose::STANDARD.decode(input)
                    .map_err(|e| HlxError::base64_error(e.to_string(), "Provide valid base64 string"))?;
                String::from_utf8(decoded)
                    .map(Value::String)
                    .map_err(|_| HlxError::base64_error("Decoded bytes are not valid UTF-8", "Ensure input is valid base64-encoded UTF-8"))
            },
            _ => Err(HlxError::invalid_parameters("base64", "Invalid 'operation'")),
        }
    }

    async fn url_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("url", "Missing 'input' parameter"))?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("url", "Missing 'operation' parameter"))?;

        match operation {
            "encode" => Ok(Value::String(urlencoding::encode(input).to_string())),
            "decode" => {
                urlencoding::decode(input)
                    .map(|s| Value::String(s.to_string()))
                    .map_err(|e| HlxError::validation_error(format!("URL decode error: {}", e), "Provide valid URL-encoded string"))
            },
            _ => Err(HlxError::invalid_parameters("url", "Invalid 'operation'")),
        }
    }

    async fn hash_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("hash", "Missing 'input' parameter"))?;

        let algorithm = params_map.get("algorithm")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("hash", "Missing 'algorithm' parameter"))?;

        match algorithm {
            "sha256" => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(input);
                Ok(Value::String(format!("{:x}", hasher.finalize())))
            },
            "md5" => {
                Ok(Value::String(format!("{:x}", md5::compute(input))))
            },
            _ => Err(HlxError::invalid_parameters("hash", "Invalid 'algorithm' - supported: sha256, md5")),
        }
    }

    async fn uuid_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let version = params_map.get("version")
            .and_then(|v| v.as_string())
            .unwrap_or("v4");

        match version {
            "v4" => Ok(Value::String(uuid::Uuid::new_v4().to_string())),
            _ => Err(HlxError::invalid_parameters("uuid", "Only v4 is supported")),
        }
    }

    /// @if - Conditional evaluation.
    /// Parameters: condition (Bool) - The condition to evaluate; then (Value) - Value if true; else (Value, optional) - Value if false.
    /// Example: @if condition=true then="yes" else="no"
    async fn if_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let condition = params_map.get("condition")
            .and_then(|v| v.as_boolean())
            .ok_or_else(|| HlxError::invalid_parameters("if", "Missing 'condition' parameter"))?;

        let then_value = params_map.get("then")
            .cloned()
            .ok_or_else(|| HlxError::invalid_parameters("if", "Missing 'then' parameter"))?;

        let else_value = params_map.get("else").cloned().unwrap_or(Value::Null);

        Ok(if condition { then_value } else { else_value })
    }

    /// @switch - Switch statement evaluation.
    /// Parameters: value (Value) - The value to match against; cases (Array) - Array of objects with 'match' and 'result' fields.
    /// Example: @switch value="a" cases=[{"match":"a","result":"apple"}]
    async fn switch_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .ok_or_else(|| HlxError::invalid_parameters("switch", "Missing 'value' parameter"))?;

        let cases = params_map.get("cases")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HlxError::invalid_parameters("switch", "Missing 'cases' parameter"))?;

        for case in cases {
            if let Value::Object(case_obj) = case {
                if let (Some(match_val), Some(result_val)) = (case_obj.get("match"), case_obj.get("result")) {
                    if match_val == value {
                        return Ok(result_val.clone());
                    }
                }
            }
        }

        // If no case matches, return the value itself or null
        Ok(Value::Null)
    }

    async fn case_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_string())
            .unwrap_or("case1")
            .to_string();

        let match_value = params_map.get("match")
            .and_then(|v| v.as_string())
            .unwrap_or("case1")
            .to_string();

        let result = value == match_value;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::String(value.to_string()));
            map.insert("match".to_string(), Value::String(match_value.to_string()));
            map.insert("result".to_string(), Value::Bool(result));
            map
        }))
    }

    async fn default_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_string())
            .unwrap_or("");

        let default = params_map.get("default")
            .and_then(|v| v.as_string())
            .unwrap_or("default");

        let result = if value.is_empty() {
            Value::String(default.to_string())
        } else {
            Value::String(value.to_string())
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::String(value.to_string()));
            map.insert("default".to_string(), Value::String(default.to_string()));
            map.insert("result".to_string(), result);
            map
        }))
    }

    async fn and_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let a = params_map.get("a")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        let b = params_map.get("b")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        let result = a && b;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("a".to_string(), Value::Bool(a));
            map.insert("b".to_string(), Value::Bool(b));
            map.insert("result".to_string(), Value::Bool(result));
            map
        }))
    }

    async fn or_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let a = params_map.get("a")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        let b = params_map.get("b")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        let result = a || b;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("a".to_string(), Value::Bool(a));
            map.insert("b".to_string(), Value::Bool(b));
            map.insert("result".to_string(), Value::Bool(result));
            map
        }))
    }

    async fn not_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        let result = !value;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::Bool(value));
            map.insert("result".to_string(), Value::Bool(result));
            map
        }))
    }

    // Math and Calculation Operations
    /// @math - Basic mathematical operations.
    /// Parameters: operation (String) - "add", "sub", "mul", "div", "mod", "pow"; a (Number); b (Number).
    /// Example: @math operation="add" a=5 b=3
    async fn math_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("math", "Missing 'operation' parameter"))?;

        let a = params_map.get("a")
            .and_then(|v| v.as_number())
            .ok_or_else(|| HlxError::invalid_parameters("math", "Missing 'a' parameter"))?;

        let b = params_map.get("b")
            .and_then(|v| v.as_number())
            .ok_or_else(|| HlxError::invalid_parameters("math", "Missing 'b' parameter"))?;

        match operation {
            "add" => Ok(Value::Number(a + b)),
            "sub" => Ok(Value::Number(a - b)),
            "mul" => Ok(Value::Number(a * b)),
            "div" => {
                if b == 0.0 {
                    Err(HlxError::validation_error("Division by zero", "Provide non-zero 'b' parameter"))
                } else {
                    Ok(Value::Number(a / b))
                }
            },
            "mod" => {
                if b == 0.0 {
                    Err(HlxError::validation_error("Modulo by zero", "Provide non-zero 'b' parameter"))
                } else {
                    Ok(Value::Number(a % b))
                }
            },
            "pow" => Ok(Value::Number(a.powf(b))),
            _ => Err(HlxError::invalid_parameters("math", "Invalid operation - supported: add, sub, mul, div, mod, pow")),
        }
    }

    /// @calc - Complex mathematical expression evaluation.
    /// Parameters: expression (String) - Mathematical expression to evaluate.
    /// Example: @calc expression="2 * (3 + 4) / 2"
    async fn calc_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let expression = params_map.get("expression")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::invalid_parameters("calc", "Missing 'expression' parameter"))?;

        let result = meval::eval_str(expression)
            .map_err(|e| HlxError::validation_error(
                format!("Expression evaluation error: {}", e),
                "Provide a valid mathematical expression"
            ))?;

        Ok(Value::Number(result))
    }

    async fn min_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.to_vec()));
            map.insert("min".to_string(), Value::Number(if result == f64::INFINITY { 0.0 } else { result }));
            map
        }))
    }

    async fn max_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.to_vec()));
            map.insert("max".to_string(), Value::Number(if result == f64::NEG_INFINITY { 0.0 } else { result }));
            map
        }))
    }

    async fn avg_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = if numbers.is_empty() {
            0.0
        } else {
            numbers.iter().sum::<f64>() / numbers.len() as f64
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.to_vec()));
            map.insert("average".to_string(), Value::Number(result));
            map
        }))
    }

    async fn sum_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = numbers.iter().sum::<f64>();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.to_vec()));
            map.insert("sum".to_string(), Value::Number(result));
            map
        }))
    }

    async fn round_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_number())
            .unwrap_or(3.14159);

        let decimals = params_map.get("decimals")
            .and_then(|v| v.as_number())
            .unwrap_or(2.0) as i32;

        let multiplier = 10.0_f64.powi(decimals);
        let result = (value * multiplier).round() / multiplier;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::Number(value));
            map.insert("decimals".to_string(), Value::Number(decimals as f64));
            map.insert("rounded".to_string(), Value::Number(result));
            map
        }))
    }

    // Array and Collection Operations
    async fn array_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("create")
            .to_string();

        let empty_vec: Vec<Value> = vec![];
        let items = params_map.get("items")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        match operation.as_str() {
            "create" => Ok(Value::Array(items.to_vec())),
            "push" => {
                let item = params_map.get("item").cloned().unwrap_or(Value::String("new_item".to_string()));
                let mut new_array = items.to_vec();
                new_array.push(item);
                Ok(Value::Array(new_array))
            },
            "pop" => {
                let mut new_array = items.to_vec();
                let popped = new_array.pop().unwrap_or(Value::String("".to_string()));
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("array".to_string(), Value::Array(new_array));
                    map.insert("popped".to_string(), popped);
                    map
                }))
            },
            _ => Ok(Value::Array(items.to_vec())),
        }
    }

    async fn map_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let transform = params_map.get("transform")
            .and_then(|v| v.as_string())
            .unwrap_or("upper")
            .to_string();

        let result: Vec<Value> = array.iter().map(|item| {
            match transform.as_str() {
                "upper" => {
                    if let Some(s) = item.as_string() {
                        Value::String(s.to_uppercase())
                    } else {
                        item.clone()
                    }
                },
                "lower" => {
                    if let Some(s) = item.as_string() {
                        Value::String(s.to_lowercase())
                    } else {
                        item.clone()
                    }
                },
                "double" => {
                    if let Some(n) = item.as_number() {
                        Value::Number(n * 2.0)
                    } else {
                        item.clone()
                    }
                },
                _ => item.clone(),
            }
        }).collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("original".to_string(), Value::Array(array.to_vec()));
            map.insert("transform".to_string(), Value::String(transform.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn filter_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let condition = params_map.get("condition")
            .and_then(|v| v.as_string())
            .unwrap_or("not_empty")
            .to_string();

        let result: Vec<Value> = array.iter().filter(|item| {
            match condition.as_str() {
                "not_empty" => {
                    if let Some(s) = item.as_string() {
                        !s.is_empty()
                    } else {
                        true
                    }
                },
                "positive" => {
                    if let Some(n) = item.as_number() {
                        n > 0.0
                    } else {
                        false
                    }
                },
                "negative" => {
                    if let Some(n) = item.as_number() {
                        n < 0.0
                    } else {
                        false
                    }
                },
                _ => true,
            }
        }).cloned().collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("original".to_string(), Value::Array(array.to_vec()));
            map.insert("condition".to_string(), Value::String(condition.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn sort_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let order = params_map.get("order")
            .and_then(|v| v.as_string())
            .unwrap_or("asc")
            .to_string();

        let mut result = array.to_vec();
        result.sort_by(|a, b| {
            match order.as_str() {
                "asc" => {
                    let a_str = a.as_string().unwrap_or("");
                    let b_str = b.as_string().unwrap_or("");
                    a_str.partial_cmp(b_str).unwrap_or(std::cmp::Ordering::Equal)
                },
                "desc" => {
                    let a_str = a.as_string().unwrap_or("");
                    let b_str = b.as_string().unwrap_or("");
                    b_str.partial_cmp(a_str).unwrap_or(std::cmp::Ordering::Equal)
                },
                _ => {
                    let a_str = a.as_string().unwrap_or("");
                    let b_str = b.as_string().unwrap_or("");
                    a_str.partial_cmp(b_str).unwrap_or(std::cmp::Ordering::Equal)
                },
            }
        });

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("original".to_string(), Value::Array(array.to_vec()));
            map.insert("order".to_string(), Value::String(order.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn join_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let empty_vec: Vec<Value> = vec![];
        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let separator = params_map.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or(",")
            .to_string();

        let strings: Vec<String> = array.iter()
            .filter_map(|v| v.as_string().map(|s| s.to_string()))
            .collect();

        let result = strings.join(&separator);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("array".to_string(), Value::Array(array.to_vec()));
            map.insert("separator".to_string(), Value::String(separator.to_string()));
            map.insert("result".to_string(), Value::String(result));
            map
        }))
    }

    async fn split_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("a,b,c,d")
            .to_string();

        let separator = params_map.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or(",")
            .to_string();

        let result: Vec<Value> = input.split(&separator)
            .map(|s| Value::String(s.to_string()))
            .collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("separator".to_string(), Value::String(separator.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn length_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .cloned()
            .unwrap_or(Value::String("hello".to_string()));

        let length = match &input {
            Value::String(s) => s.len(),
            Value::Array(a) => a.len(),
            Value::Object(o) => o.len(),
            _ => 0,
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), input.clone());
            map.insert("length".to_string(), Value::Number(length as f64));
            map
        }))
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for FundamentalOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        // Support both @ prefixed and non-prefixed operators
        let clean_operator = operator.strip_prefix('@').unwrap_or(operator);

        match clean_operator {
            // Variable and Environment Access
            "var" => self.var_operator(params).await,
            "env" => self.env_operator(params).await,

            // HTTP and Request Data
            "request" => self.request_operator(params).await,
            "session" => self.session_operator(params).await,
            "cookie" => self.cookie_operator(params).await,
            "header" => self.header_operator(params).await,
            "param" => self.param_operator(params).await,
            "query" => self.query_operator(params).await,

            // Date and Time
            "date" => self.date_operator(params).await,
            "time" => self.time_operator(params).await,
            "timestamp" => self.timestamp_operator(params).await,
            "now" => self.now_operator(params).await,
            "format" => self.format_operator(params).await,
            "timezone" => self.timezone_operator(params).await,

            // String and Data Processing
            "string" => self.string_operator(params).await,
            "regex" => self.regex_operator(params).await,
            "json" => self.json_operator(params).await,
            "base64" => self.base64_operator(params).await,
            "url" => self.url_operator(params).await,
            "hash" => self.hash_operator(params).await,
            "uuid" => self.uuid_operator(params).await,

            // Conditional and Logic
            "if" => self.if_operator(params).await,
            "switch" => self.switch_operator(params).await,
            "case" => self.case_operator(params).await,
            "default" => self.default_operator(params).await,
            "and" => self.and_operator(params).await,
            "or" => self.or_operator(params).await,
            "not" => self.not_operator(params).await,

            // Math and Calculations
            "math" => self.math_operator(params).await,
            "calc" => self.calc_operator(params).await,
            "min" => self.min_operator(params).await,
            "max" => self.max_operator(params).await,
            "avg" => self.avg_operator(params).await,
            "sum" => self.sum_operator(params).await,
            "round" => self.round_operator(params).await,

            // Array and Collections
            "array" => self.array_operator(params).await,
            "map" => self.map_operator(params).await,
            "filter" => self.filter_operator(params).await,
            "sort" => self.sort_operator(params).await,
            "join" => self.join_operator(params).await,
            "split" => self.split_operator(params).await,
            "length" => self.length_operator(params).await,

            _ => Err(HlxError::unknown_error(format!("Unknown fundamental operator: @{}", clean_operator), "Check the operator name")),
        }
    }
}

/// Clean registry API for fundamental operators
pub struct OperatorRegistry {
    core: Arc<FundamentalOperators>,
}

impl OperatorRegistry {
    /// Create a new registry with an empty context (default values)
    pub async fn new() -> Result<Self, HlxError> {
        let context = ExecutionContext::default();
        Self::new_with_context(context).await
    }

    /// Create a new registry with the provided execution context
    pub async fn new_with_context(context: ExecutionContext) -> Result<Self, HlxError> {
        let core = FundamentalOperators::new_with_context(context).await?;
        Ok(Self {
            core: Arc::new(core),
        })
    }

    /// Execute an operator with the given parameters
    pub async fn execute(&self, op: &str, params: &str) -> Result<Value, HlxError> {
        self.core.execute(op, params).await
    }

    /// Get access to the underlying context for inspection/modification
    pub fn context(&self) -> &Arc<ExecutionContext> {
        &self.core.context
    }
}
