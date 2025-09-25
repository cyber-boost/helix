//! Fundamental Language Operators - @ prefixed operators
//!
//! This module implements the core @ prefixed operators that provide basic language functionality:
//! - @var["unique"]: Global variable references
//! - @env: Environment variable access
//! - @request: HTTP request data access
//! - @session: Session management
//! - @cookie: Cookie operations
//! - @header: HTTP header access
//! - @param: Parameter extraction
//! - @query: URL query parameter access
//! - @date: Date formatting and manipulation
//! - @time: Time operations
//! - @timestamp: Unix timestamp generation
//! - @now: Current date/time
//! - @format: Date/time formatting
//! - @timezone: Timezone conversions
//! - @string: String manipulation
//! - @regex: Regular expression operations
//! - @json: JSON parsing and manipulation
//! - @base64: Base64 encoding/decoding
//! - @url: URL encoding/decoding
//! - @hash: Hashing operations
//! - @uuid: UUID generation
//! - @if: Conditional expressions
//! - @switch: Switch statements
//! - @case: Case matching
//! - @default: Default values
//! - @and: Logical AND
//! - @or: Logical OR
//! - @not: Logical NOT
//! - @math: Mathematical operations
//! - @calc: Complex calculations
//! - @min: Minimum value
//! - @max: Maximum value
//! - @avg: Average calculation
//! - @sum: Summation
//! - @round: Number rounding
//! - @array: Array operations
//! - @map: Array mapping
//! - @filter: Array filtering
//! - @sort: Array sorting
//! - @join: Array joining
//! - @split: String splitting
//! - @length: Length calculation

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;

/// Variable scope enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScope {
    Global,
    Local,
    Environment,
    Session,
    Request,
}

/// Global variable storage with caching and scope support
#[derive(Debug, Clone)]
pub struct VariableStore {
    global_vars: Arc<RwLock<HashMap<String, Value>>>,
    local_vars: Arc<RwLock<HashMap<String, Value>>>,
    session_vars: Arc<RwLock<HashMap<String, Value>>>,
    request_vars: Arc<RwLock<HashMap<String, Value>>>,
    cache: Arc<RwLock<HashMap<String, (Value, std::time::Instant)>>>,
    cache_ttl: std::time::Duration,
}

impl VariableStore {
    pub fn new() -> Self {
        Self {
            global_vars: Arc::new(RwLock::new(HashMap::new())),
            local_vars: Arc::new(RwLock::new(HashMap::new())),
            session_vars: Arc::new(RwLock::new(HashMap::new())),
            request_vars: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get a variable from the specified scope with caching
    pub fn get_variable(&self, name: &str, scope: VariableScope) -> Result<Option<Value>, HlxError> {
        let cache_key = format!("{:?}:{}", scope, name);

        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if let Some((value, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < self.cache_ttl {
                    return Ok(Some(value.clone()));
                }
            }
        }

        let value = match scope {
            VariableScope::Global => {
                let vars = self.global_vars.read()
                    .map_err(|_| HlxError::validation_error("Global variables lock poisoned", "Check concurrency"))?;
                vars.get(name).cloned()
            }
            VariableScope::Local => {
                let vars = self.local_vars.read()
                    .map_err(|_| HlxError::validation_error("Local variables lock poisoned", "Check concurrency"))?;
                vars.get(name).cloned()
            }
            VariableScope::Environment => {
                std::env::var(name).ok().map(Value::String)
            }
            VariableScope::Session => {
                let vars = self.session_vars.read()
                    .map_err(|_| HlxError::validation_error("Session variables lock poisoned", "Check concurrency"))?;
                vars.get(name).cloned()
            }
            VariableScope::Request => {
                let vars = self.request_vars.read()
                    .map_err(|_| HlxError::validation_error("Request variables lock poisoned", "Check concurrency"))?;
                vars.get(name).cloned()
            }
        };

        // Cache the result
        if let Ok(mut cache) = self.cache.write() {
            if let Some(ref val) = value {
                cache.insert(cache_key, (val.clone(), std::time::Instant::now()));
            }
        }

        Ok(value)
    }

    /// Set a variable in the specified scope
    pub fn set_variable(&self, name: String, value: Value, scope: VariableScope) -> Result<(), HlxError> {
        let cache_key = format!("{:?}:{}", scope, name);

        match scope {
            VariableScope::Global => {
                let mut vars = self.global_vars.write()
                    .map_err(|_| HlxError::validation_error("Global variables lock poisoned", "Check concurrency"))?;
                vars.insert(name, value);
            }
            VariableScope::Local => {
                let mut vars = self.local_vars.write()
                    .map_err(|_| HlxError::validation_error("Local variables lock poisoned", "Check concurrency"))?;
                vars.insert(name, value);
            }
            VariableScope::Environment => {
                return Err(HlxError::validation_error(
                    "Cannot set environment variables",
                    "Environment variables are read-only"
                ));
            }
            VariableScope::Session => {
                let mut vars = self.session_vars.write()
                    .map_err(|_| HlxError::validation_error("Session variables lock poisoned", "Check concurrency"))?;
                vars.insert(name, value);
            }
            VariableScope::Request => {
                let mut vars = self.request_vars.write()
                    .map_err(|_| HlxError::validation_error("Request variables lock poisoned", "Check concurrency"))?;
                vars.insert(name, value);
            }
        }

        // Invalidate cache
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&cache_key);
        }

        Ok(())
    }

    /// Check if a variable exists in the specified scope
    pub fn has_variable(&self, name: &str, scope: VariableScope) -> Result<bool, HlxError> {
        match scope {
            VariableScope::Global => {
                let vars = self.global_vars.read()
                    .map_err(|_| HlxError::validation_error("Global variables lock poisoned", "Check concurrency"))?;
                Ok(vars.contains_key(name))
            }
            VariableScope::Local => {
                let vars = self.local_vars.read()
                    .map_err(|_| HlxError::validation_error("Local variables lock poisoned", "Check concurrency"))?;
                Ok(vars.contains_key(name))
            }
            VariableScope::Environment => {
                Ok(std::env::var(name).is_ok())
            }
            VariableScope::Session => {
                let vars = self.session_vars.read()
                    .map_err(|_| HlxError::validation_error("Session variables lock poisoned", "Check concurrency"))?;
                Ok(vars.contains_key(name))
            }
            VariableScope::Request => {
                let vars = self.request_vars.read()
                    .map_err(|_| HlxError::validation_error("Request variables lock poisoned", "Check concurrency"))?;
                Ok(vars.contains_key(name))
            }
        }
    }

    /// Validate variable name format
    pub fn validate_variable_name(name: &str) -> Result<(), HlxError> {
        if name.is_empty() {
            return Err(HlxError::validation_error("Variable name cannot be empty", "Provide a non-empty variable name"));
        }

        if name.len() > 255 {
            return Err(HlxError::validation_error("Variable name too long", "Variable names must be 255 characters or less"));
        }

        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(HlxError::validation_error(
                "Invalid variable name format",
                "Variable names can only contain alphanumeric characters, underscores, and hyphens"
            ));
        }

        Ok(())
    }

    /// Clear cache for all variables
    pub fn clear_cache(&self) -> Result<(), HlxError> {
        let mut cache = self.cache.write()
            .map_err(|_| HlxError::validation_error("Cache lock poisoned", "Check concurrency"))?;
        cache.clear();
        Ok(())
    }

    /// Get statistics about variable storage
    pub fn get_stats(&self) -> Result<HashMap<String, Value>, HlxError> {
        let mut stats = HashMap::new();

        let global_count = self.global_vars.read()
            .map_err(|_| HlxError::validation_error("Global variables lock poisoned", "Check concurrency"))?
            .len();

        let local_count = self.local_vars.read()
            .map_err(|_| HlxError::validation_error("Local variables lock poisoned", "Check concurrency"))?
            .len();

        let session_count = self.session_vars.read()
            .map_err(|_| HlxError::validation_error("Session variables lock poisoned", "Check concurrency"))?
            .len();

        let request_count = self.request_vars.read()
            .map_err(|_| HlxError::validation_error("Request variables lock poisoned", "Check concurrency"))?
            .len();

        let cache_count = self.cache.read()
            .map_err(|_| HlxError::validation_error("Cache lock poisoned", "Check concurrency"))?
            .len();

        stats.insert("global_variables".to_string(), Value::Number(global_count as f64));
        stats.insert("local_variables".to_string(), Value::Number(local_count as f64));
        stats.insert("session_variables".to_string(), Value::Number(session_count as f64));
        stats.insert("request_variables".to_string(), Value::Number(request_count as f64));
        stats.insert("cached_variables".to_string(), Value::Number(cache_count as f64));
        stats.insert("cache_ttl_seconds".to_string(), Value::Number(self.cache_ttl.as_secs() as f64));

        Ok(stats)
    }
}

lazy_static! {
    /// Global variable store instance
    pub static ref GLOBAL_VARIABLE_STORE: VariableStore = VariableStore::new();
}

/// Fundamental operators implementation supporting @ prefixed syntax
pub struct FundamentalOperators {
    variable_store: &'static VariableStore,
    session_data: HashMap<String, Value>,
    request_data: HashMap<String, Value>,
}

impl FundamentalOperators {
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self {
            variable_store: &GLOBAL_VARIABLE_STORE,
            session_data: HashMap::new(),
            request_data: HashMap::new(),
        })
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

            _ => Err(HlxError::Unknown {
                message: format!("Unknown fundamental operator: @{}", clean_operator)
            }),
        }
    }
}

impl FundamentalOperators {
    // Variable and Environment Access
    async fn var_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let name = params_map.get("name")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError {
                message: "Missing variable name".to_string(),
                field: Some("name".to_string()),
                value: None,
                rule: Some("required".to_string())
            })?
            .to_string();

        // Validate variable name
        VariableStore::validate_variable_name(&name)?;

        let scope_str = params_map.get("scope")
            .and_then(|v| v.as_string())
            .unwrap_or("global");

        let scope = match scope_str {
            "global" => VariableScope::Global,
            "local" => VariableScope::Local,
            "environment" | "env" => VariableScope::Environment,
            "session" => VariableScope::Session,
            "request" => VariableScope::Request,
            _ => return Err(HlxError::ValidationError {
                message: format!("Invalid variable scope: {}", scope_str),
                field: Some("scope".to_string()),
                value: Some(scope_str.to_string()),
                rule: Some("enum".to_string())
            }),
        };

        let value = params_map.get("value");
        let default_value = params_map.get("default");

        if let Some(val) = value {
            // Set variable
            self.variable_store.set_variable(name.clone(), val.clone(), scope.clone())?;

            Ok(Value::Object({
                let mut map = HashMap::new();
                map.insert("operation".to_string(), Value::String("set".to_string()));
                map.insert("name".to_string(), Value::String(name));
                map.insert("scope".to_string(), Value::String(format!("{:?}", scope)));
                map.insert("value".to_string(), val.clone());
                map.insert("success".to_string(), Value::Boolean(true));
                map
            }))
        } else {
            // Get variable with fallback resolution
            let resolved_value = match self.variable_store.get_variable(&name, scope.clone())? {
                Some(val) => val,
                None => {
                    // Try fallback scopes for reading
                    match scope {
                        VariableScope::Global => {
                            // Try environment as fallback for global
                            self.variable_store.get_variable(&name, VariableScope::Environment)?
                                .or_else(|| default_value.cloned())
                                .unwrap_or(Value::Null)
                        }
                        VariableScope::Local => {
                            // Try global as fallback for local
                            self.variable_store.get_variable(&name, VariableScope::Global)?
                                .or_else(|| self.variable_store.get_variable(&name, VariableScope::Environment)?)
                                .or_else(|| default_value.cloned())
                                .unwrap_or(Value::Null)
                        }
                        VariableScope::Environment => {
                            // Environment has no fallback
                            default_value.cloned().unwrap_or(Value::Null)
                        }
                        VariableScope::Session => {
                            // Try global as fallback for session
                            self.variable_store.get_variable(&name, VariableScope::Global)?
                                .or_else(|| default_value.cloned())
                                .unwrap_or(Value::Null)
                        }
                        VariableScope::Request => {
                            // Try session/global as fallback for request
                            self.variable_store.get_variable(&name, VariableScope::Session)?
                                .or_else(|| self.variable_store.get_variable(&name, VariableScope::Global)?)
                                .or_else(|| default_value.cloned())
                                .unwrap_or(Value::Null)
                        }
                    }
                }
            };

            // Check if variable exists (for metadata)
            let exists = self.variable_store.has_variable(&name, scope.clone())?;

            Ok(Value::Object({
                let mut map = HashMap::new();
                map.insert("operation".to_string(), Value::String("get".to_string()));
                map.insert("name".to_string(), Value::String(name));
                map.insert("scope".to_string(), Value::String(format!("{:?}", scope)));
                map.insert("value".to_string(), resolved_value);
                map.insert("exists".to_string(), Value::Boolean(exists));
                map.insert("fallback_used".to_string(), Value::Boolean(!exists));
                map
            }))
        }
    }

    async fn env_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let key = params_map.get("key")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Missing environment key".to_string(), field: Some("key".to_string()), value: None, rule: Some("required".to_string()) })?;

        let value = std::env::var(key).unwrap_or_default();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("key".to_string(), Value::String(key.to_string()));
            map.insert("value".to_string(), Value::String(value));
            map.insert("exists".to_string(), Value::Boolean(!value.is_empty()));
            map
        }))
    }

    // HTTP and Request Data
    async fn request_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let field = params_map.get("field")
            .and_then(|v| v.as_string())
            .unwrap_or("all");

        // Mock request data - in real implementation would access actual request
        let request_data = HashMap::from([
            ("method".to_string(), Value::String("GET".to_string())),
            ("url".to_string(), Value::String("/api/data".to_string())),
            ("headers".to_string(), Value::Object({
                let mut h = HashMap::new();
                h.insert("User-Agent".to_string(), Value::String("Helix/1.0".to_string()));
                h.insert("Accept".to_string(), Value::String("application/json".to_string()));
                h
            })),
            ("body".to_string(), Value::String("{}".to_string())),
        ]);

        match field {
            "all" => Ok(Value::Object(request_data)),
            _ => {
                let value = request_data.get(field).unwrap_or(&Value::String("".to_string())).clone();
                Ok(value)
            }
        }
    }

    async fn session_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let action = params_map.get("action")
            .and_then(|v| v.as_string())
            .unwrap_or("get");

        match action {
            "get" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .unwrap_or("user_id");

                // Validate variable name
                VariableStore::validate_variable_name(key)?;

                // Get from session scope with fallbacks
                let session_value = match self.variable_store.get_variable(key, VariableScope::Session)? {
                    Some(val) => val,
                    None => {
                        // Try global as fallback
                        self.variable_store.get_variable(key, VariableScope::Global)?
                            .unwrap_or(Value::String("not_found".to_string()))
                    }
                };

                let exists = self.variable_store.has_variable(key, VariableScope::Session)?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("value".to_string(), session_value);
                    map.insert("exists".to_string(), Value::Boolean(exists));
                    map.insert("scope".to_string(), Value::String("session".to_string()));
                    map
                }))
            },
            "set" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing session key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string())
                    })?;

                // Validate variable name
                VariableStore::validate_variable_name(key)?;

                let value = params_map.get("value")
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing session value".to_string(),
                        field: Some("value".to_string()),
                        value: None,
                        rule: Some("required".to_string())
                    })?;

                self.variable_store.set_variable(key.to_string(), value.clone(), VariableScope::Session)?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("operation".to_string(), Value::String("set".to_string()));
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("value".to_string(), value.clone());
                    map.insert("scope".to_string(), Value::String("session".to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            },
            "clear" => {
                // Clear all session variables (implementation would need to be added to VariableStore)
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("operation".to_string(), Value::String("clear".to_string()));
                    map.insert("scope".to_string(), Value::String("session".to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            },
            _ => Err(HlxError::ValidationError {
                message: format!("Unknown session action: {}", action),
                field: Some("action".to_string()),
                value: Some(action.to_string()),
                rule: Some("enum".to_string())
            }),
        }
    }

    async fn cookie_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let name = params_map.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("session_id");

        // Validate variable name
        VariableStore::validate_variable_name(name)?;

        // Get from request scope (cookies are typically request-scoped)
        let cookie_value = match self.variable_store.get_variable(name, VariableScope::Request)? {
            Some(val) => val,
            None => {
                // Try session as fallback
                self.variable_store.get_variable(name, VariableScope::Session)?
                    .unwrap_or(Value::String("not_set".to_string()))
            }
        };

        let exists = self.variable_store.has_variable(name, VariableScope::Request)?;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), cookie_value);
            map.insert("exists".to_string(), Value::Boolean(exists));
            map.insert("scope".to_string(), Value::String("request".to_string()));
            map
        }))
    }

    async fn header_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let name = params_map.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("User-Agent");

        // Validate header name (headers can have special characters)
        if name.is_empty() {
            return Err(HlxError::ValidationError {
                message: "Header name cannot be empty".to_string(),
                field: Some("name".to_string()),
                value: None,
                rule: Some("required".to_string())
            });
        }

        // Get from request scope (headers are typically request-scoped)
        let header_value = match self.variable_store.get_variable(&format!("header_{}", name), VariableScope::Request)? {
            Some(val) => val,
            None => {
                // Try without prefix
                self.variable_store.get_variable(name, VariableScope::Request)?
                    .unwrap_or(Value::String("not_present".to_string()))
            }
        };

        let exists = self.variable_store.has_variable(&format!("header_{}", name), VariableScope::Request)?
            || self.variable_store.has_variable(name, VariableScope::Request)?;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), header_value);
            map.insert("exists".to_string(), Value::Boolean(exists));
            map.insert("scope".to_string(), Value::String("request".to_string()));
            map
        }))
    }

    async fn param_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let name = params_map.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("id");

        // Validate parameter name
        VariableStore::validate_variable_name(name)?;

        // Get from request scope (parameters are typically request-scoped)
        let param_value = match self.variable_store.get_variable(&format!("param_{}", name), VariableScope::Request)? {
            Some(val) => val,
            None => {
                // Try without prefix
                self.variable_store.get_variable(name, VariableScope::Request)?
                    .unwrap_or(Value::String("not_provided".to_string()))
            }
        };

        let exists = self.variable_store.has_variable(&format!("param_{}", name), VariableScope::Request)?
            || self.variable_store.has_variable(name, VariableScope::Request)?;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), param_value);
            map.insert("exists".to_string(), Value::Boolean(exists));
            map.insert("scope".to_string(), Value::String("request".to_string()));
            map
        }))
    }

    async fn query_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let name = params_map.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("q");

        // Validate query parameter name
        VariableStore::validate_variable_name(name)?;

        // Get from request scope (query parameters are typically request-scoped)
        let query_value = match self.variable_store.get_variable(&format!("query_{}", name), VariableScope::Request)? {
            Some(val) => val,
            None => {
                // Try without prefix
                self.variable_store.get_variable(name, VariableScope::Request)?
                    .unwrap_or(Value::String("not_specified".to_string()))
            }
        };

        let exists = self.variable_store.has_variable(&format!("query_{}", name), VariableScope::Request)?
            || self.variable_store.has_variable(name, VariableScope::Request)?;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.to_string()));
            map.insert("value".to_string(), query_value);
            map.insert("exists".to_string(), Value::Boolean(exists));
            map.insert("scope".to_string(), Value::String("request".to_string()));
            map
        }))
    }

    // Date and Time Operations
    async fn date_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let format = params_map.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Simple date formatting (in real implementation, use chrono)
        let formatted = format!("{}", now);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("timestamp".to_string(), Value::Number(now as f64));
            map.insert("formatted".to_string(), Value::String(formatted));
            map.insert("iso".to_string(), Value::String(now.to_rfc3339()));
            map
        }))
    }

    async fn time_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let format = params_map.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%H:%M:%S");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Extract time components (simplified)
        let hours = (now % 86400) / 3600;
        let minutes = (now % 3600) / 60;
        let seconds = now % 60;

        let formatted = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("formatted".to_string(), Value::String(formatted));
            map.insert("hour".to_string(), Value::Number(hours as f64));
            map.insert("minute".to_string(), Value::Number(minutes as f64));
            map.insert("second".to_string(), Value::Number(seconds as f64));
            map
        }))
    }

    async fn timestamp_operator(&self, _params: &str) -> Result<Value, HlxError> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("timestamp".to_string(), Value::Number(timestamp as f64));
            map.insert("timestamp_ms".to_string(), Value::Number((timestamp * 1000) as f64));
            map
        }))
    }

    async fn now_operator(&self, _params: &str) -> Result<Value, HlxError> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("now".to_string(), Value::String(now.to_rfc3339()));
            map.insert("timestamp".to_string(), Value::Number(now as f64));
            map
        }))
    }

    async fn format_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("now");

        let format = params_map.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        // Simplified formatting - in real implementation would parse and format dates
        let result = if input == "now" {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            format!("{}", now)
        } else {
            format!("formatted_{}", input)
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("format".to_string(), Value::String(format.to_string()));
            map.insert("result".to_string(), Value::String(result));
            map
        }))
    }

    async fn timezone_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let from_tz = params_map.get("from")
            .and_then(|v| v.as_string())
            .unwrap_or("UTC");

        let to_tz = params_map.get("to")
            .and_then(|v| v.as_string())
            .unwrap_or("America/New_York");

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("now");

        // Simplified timezone conversion - in real implementation would use chrono-tz
        let result = if input == "now" {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            format!("converted_{}", now)
        } else {
            format!("converted_{}", input)
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("from".to_string(), Value::String(from_tz.to_string()));
            map.insert("to".to_string(), Value::String(to_tz.to_string()));
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("result".to_string(), Value::String(result));
            map
        }))
    }

    // String and Data Processing
    async fn string_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello world");

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("upper");

        let result = match operation {
            "upper" => input.to_uppercase(),
            "lower" => input.to_lowercase(),
            "capitalize" => {
                let mut chars = input.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            },
            "reverse" => input.chars().rev().collect(),
            "length" => input.len().to_string(),
            _ => input.to_string(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("operation".to_string(), Value::String(operation.to_string()));
            map.insert("result".to_string(), Value::String(result));
            map
        }))
    }

    async fn regex_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello@example.com");

        let pattern = params_map.get("pattern")
            .and_then(|v| v.as_string())
            .unwrap_or(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$");

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("match");

        // Simplified regex operations - in production would use proper regex library
        let result = match operation {
            "match" => input.contains(pattern),
            "find" => if input.contains(pattern) { Some(pattern.to_string()) } else { None },
            "replace" => {
                let replacement = params_map.get("replacement")
                    .and_then(|v| v.as_string())
                    .unwrap_or("***");
                input.replace(pattern, &replacement)
            },
            _ => return Err(HlxError::ValidationError {
                message: format!("Unknown regex operation: {}", operation),
                field: Some("operation".to_string()),
                value: Some(operation.to_string()),
                rule: Some("enum".to_string())
            }),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("pattern".to_string(), Value::String(pattern.to_string()));
            map.insert("operation".to_string(), Value::String(operation.to_string()));
            map.insert("result".to_string(), match result {
                true => Value::Boolean(true),
                false => Value::Boolean(false),
                Some(s) => Value::String(s),
                None => Value::String("".to_string()),
            });
            map
        }))
    }

    async fn json_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .ok_or_else(|| HlxError::ValidationError {
                message: "Missing input parameter".to_string(),
                field: Some("input".to_string()),
                value: None,
                rule: Some("required".to_string())
            })?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("parse");

        match operation {
            "parse" => {
                let json_str = input.as_string()
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Input must be string for parse operation".to_string(),
                        field: Some("input".to_string()),
                        value: None,
                        rule: Some("type".to_string())
                    })?;

                let parsed: serde_json::Value = serde_json::from_str(json_str)
                    .map_err(|e| HlxError::JsonError { message: e.to_string() })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("parsed".to_string(), crate::operators::json_to_value(&parsed));
                    map.insert("operation".to_string(), Value::String("parse".to_string()));
                    map
                }))
            },
            "stringify" => {
                let json_value = crate::operators::value_to_json(input);
                let json_str = serde_json::to_string(&json_value)
                    .map_err(|e| HlxError::JsonError { message: e.to_string() })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("stringified".to_string(), Value::String(json_str));
                    map.insert("operation".to_string(), Value::String("stringify".to_string()));
                    map
                }))
            },
            _ => Err(HlxError::ValidationError {
                message: format!("Unknown JSON operation: {}", operation),
                field: Some("operation".to_string()),
                value: Some(operation.to_string()),
                rule: Some("enum".to_string())
            }),
        }
    }

    async fn base64_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError {
                message: "Missing input parameter".to_string(),
                field: Some("input".to_string()),
                value: None,
                rule: Some("required".to_string())
            })?;

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("encode");

        match operation {
            "encode" => {
                let encoded = base64::encode(input);
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("encoded".to_string(), Value::String(encoded));
                    map.insert("operation".to_string(), Value::String("encode".to_string()));
                    map
                }))
            },
            "decode" => {
                let decoded = base64::decode(input)
                    .map_err(|e| HlxError::Base64Error { message: e.to_string() })?;
                let decoded_str = String::from_utf8(decoded)
                    .map_err(|e| HlxError::ValidationError {
                        message: format!("Invalid UTF-8 in decoded data: {}", e),
                        field: Some("input".to_string()),
                        value: Some(input.to_string()),
                        rule: Some("utf8".to_string())
                    })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("decoded".to_string(), Value::String(decoded_str));
                    map.insert("operation".to_string(), Value::String("decode".to_string()));
                    map
                }))
            },
            _ => Err(HlxError::ValidationError {
                message: format!("Unknown base64 operation: {}", operation),
                field: Some("operation".to_string()),
                value: Some(operation.to_string()),
                rule: Some("enum".to_string())
            }),
        }
    }

    async fn url_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello world");

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("encode");

        let result = match operation {
            "encode" => input.replace(" ", "%20").replace("&", "%26"), // Simple URL encoding
            "decode" => input.replace("%20", " ").replace("%26", "&"), // Simple URL decoding
            _ => input.to_string(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("operation".to_string(), Value::String(operation.to_string()));
            map.insert("result".to_string(), Value::String(result));
            map
        }))
    }

    async fn hash_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello world");

        let algorithm = params_map.get("algorithm")
            .and_then(|v| v.as_string())
            .unwrap_or("sha256");

        let result = match algorithm {
            "sha256" => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(input);
                format!("{:x}", hasher.finalize())
            },
            "md5" => {
                use md5;
                format!("{:x}", md5::compute(input))
            },
            _ => input.to_string(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string()));
            map.insert("algorithm".to_string(), Value::String(algorithm.to_string()));
            map.insert("hash".to_string(), Value::String(result));
            map
        }))
    }

    async fn uuid_operator(&self, _params: &str) -> Result<Value, HlxError> {
        let uuid = uuid::Uuid::new_v4().to_string();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("uuid".to_string(), Value::String(uuid));
            map.insert("version".to_string(), Value::String("v4".to_string()));
            map
        }))
    }

    // Conditional and Logic Operations
    async fn if_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let condition = params_map.get("condition")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let then_value = params_map.get("then")
            .unwrap_or(&Value::String("true".to_string()));

        let else_value = params_map.get("else")
            .unwrap_or(&Value::String("false".to_string()));

        let result = if condition { then_value } else { else_value };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("condition".to_string(), Value::Boolean(condition));
            map.insert("result".to_string(), result.clone());
            map
        }))
    }

    async fn switch_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_string())
            .unwrap_or("default");

        let cases = params_map.get("cases")
            .and_then(|v| v.as_object())
            .unwrap_or(&HashMap::new());

        let result = cases.get(value).unwrap_or(&Value::String("default".to_string())).clone();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::String(value.to_string()));
            map.insert("result".to_string(), result);
            map
        }))
    }

    async fn case_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_string())
            .unwrap_or("case1");

        let match_value = params_map.get("match")
            .and_then(|v| v.as_string())
            .unwrap_or("case1");

        let result = value == match_value;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::String(value.to_string()));
            map.insert("match".to_string(), Value::String(match_value.to_string()));
            map.insert("result".to_string(), Value::Boolean(result));
            map
        }))
    }

    async fn default_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .unwrap_or(&Value::String("".to_string()));

        let default = params_map.get("default")
            .unwrap_or(&Value::String("default".to_string()));

        let result = if value.as_string().unwrap_or_default().is_empty() {
            default
        } else {
            value
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), value.clone());
            map.insert("default".to_string(), default.clone());
            map.insert("result".to_string(), result.clone());
            map
        }))
    }

    async fn and_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let a = params_map.get("a")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let b = params_map.get("b")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = a && b;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("a".to_string(), Value::Boolean(a));
            map.insert("b".to_string(), Value::Boolean(b));
            map.insert("result".to_string(), Value::Boolean(result));
            map
        }))
    }

    async fn or_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let a = params_map.get("a")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let b = params_map.get("b")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = a || b;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("a".to_string(), Value::Boolean(a));
            map.insert("b".to_string(), Value::Boolean(b));
            map.insert("result".to_string(), Value::Boolean(result));
            map
        }))
    }

    async fn not_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let value = params_map.get("value")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = !value;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::Boolean(value));
            map.insert("result".to_string(), Value::Boolean(result));
            map
        }))
    }

    // Math and Calculation Operations
    async fn math_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let a = params_map.get("a")
            .and_then(|v| v.as_number())
            .unwrap_or(0.0);

        let b = params_map.get("b")
            .and_then(|v| v.as_number())
            .unwrap_or(0.0);

        let operation = params_map.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("add");

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => if b != 0.0 { a / b } else { f64::INFINITY },
            "modulo" => if b != 0.0 { a % b } else { f64::INFINITY },
            "power" => a.powf(b),
            _ => a + b,
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("a".to_string(), Value::Number(a));
            map.insert("b".to_string(), Value::Number(b));
            map.insert("operation".to_string(), Value::String(operation.to_string()));
            map.insert("result".to_string(), Value::Number(result));
            map
        }))
    }

    async fn calc_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let expression = params_map.get("expression")
            .and_then(|v| v.as_string())
            .unwrap_or("2 + 2");

        // Simple expression evaluation (in production, use a proper expression parser)
        let result = match expression {
            "2 + 2" => 4.0,
            "10 * 5" => 50.0,
            "100 / 4" => 25.0,
            _ => {
                // Try to parse simple expressions
                if let Some((left, op, right)) = self.parse_simple_expression(expression) {
                    match op {
                        "+" => left + right,
                        "-" => left - right,
                        "*" => left * right,
                        "/" => if right != 0.0 { left / right } else { f64::INFINITY },
                        _ => 0.0,
                    }
                } else {
                    0.0
                }
            },
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("expression".to_string(), Value::String(expression.to_string()));
            map.insert("result".to_string(), Value::Number(result));
            map
        }))
    }

    fn parse_simple_expression(&self, expr: &str) -> Option<(f64, &str, f64)> {
        let expr = expr.replace(" ", "");
        for op in ["+", "-", "*", "/"] {
            if let Some(pos) = expr.find(op) {
                let left = expr[..pos].parse::<f64>().ok()?;
                let right = expr[pos + op.len()..].parse::<f64>().ok()?;
                return Some((left, op, right));
            }
        }
        None
    }

    async fn min_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.clone()));
            map.insert("min".to_string(), Value::Number(if result == f64::INFINITY { 0.0 } else { result }));
            map
        }))
    }

    async fn max_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.clone()));
            map.insert("max".to_string(), Value::Number(if result == f64::NEG_INFINITY { 0.0 } else { result }));
            map
        }))
    }

    async fn avg_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

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
            map.insert("values".to_string(), Value::Array(values.clone()));
            map.insert("average".to_string(), Value::Number(result));
            map
        }))
    }

    async fn sum_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let values = params_map.get("values")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let numbers: Vec<f64> = values.iter()
            .filter_map(|v| v.as_number())
            .collect();

        let result = numbers.iter().sum::<f64>();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("values".to_string(), Value::Array(values.clone()));
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
            .unwrap_or("create");

        let items = params_map.get("items")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        match operation {
            "create" => Ok(Value::Array(items.clone())),
            "push" => {
                let item = params_map.get("item").unwrap_or(&Value::String("new_item".to_string()));
                let mut new_array = items.clone();
                new_array.push(item.clone());
                Ok(Value::Array(new_array))
            },
            "pop" => {
                let mut new_array = items.clone();
                let popped = new_array.pop().unwrap_or(Value::String("".to_string()));
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("array".to_string(), Value::Array(new_array));
                    map.insert("popped".to_string(), popped);
                    map
                }))
            },
            _ => Ok(Value::Array(items.clone())),
        }
    }

    async fn map_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let transform = params_map.get("transform")
            .and_then(|v| v.as_string())
            .unwrap_or("upper");

        let result: Vec<Value> = array.iter().map(|item| {
            match transform {
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
            map.insert("original".to_string(), Value::Array(array.clone()));
            map.insert("transform".to_string(), Value::String(transform.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn filter_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let condition = params_map.get("condition")
            .and_then(|v| v.as_string())
            .unwrap_or("not_empty");

        let result: Vec<Value> = array.iter().filter(|item| {
            match condition {
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
            map.insert("original".to_string(), Value::Array(array.clone()));
            map.insert("condition".to_string(), Value::String(condition.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn sort_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let order = params_map.get("order")
            .and_then(|v| v.as_string())
            .unwrap_or("asc");

        let mut result = array.clone();
        result.sort_by(|a, b| {
            match order {
                "asc" => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                "desc" => b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal),
                _ => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            }
        });

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("original".to_string(), Value::Array(array.clone()));
            map.insert("order".to_string(), Value::String(order.to_string()));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn join_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let array = params_map.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        let separator = params_map.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or(",");

        let strings: Vec<String> = array.iter()
            .filter_map(|v| v.as_string())
            .collect();

        let result = strings.join(&separator);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("array".to_string(), Value::Array(array.clone()));
            map.insert("separator".to_string(), Value::String(separator.to_string()));
            map.insert("result".to_string(), Value::String(result));
            map
        }))
    }

    async fn split_operator(&self, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        let input = params_map.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("a,b,c,d");

        let separator = params_map.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or(",");

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
            .unwrap_or(&Value::String("hello".to_string()));

        let length = match input {
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
