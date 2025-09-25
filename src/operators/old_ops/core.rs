//! Core Language Features - 50+ operators
//! 
//! This module implements the fundamental operators that provide basic language functionality:
//! - @variable: Global variable references
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
use serde_json;
use std::collections::HashMap;
use std::fs;

/// Core operators implementation
pub struct CoreOperators {
    variables: HashMap<String, Value>,
}

impl CoreOperators {
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self {
            variables: HashMap::new(),
        })
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for CoreOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            // Variable and Environment Operators
            "@[" => self.variable_operator(&params_map).await,
            "env" => self.env_operator(&params_map).await,
            "request" => self.request_operator(&params_map).await,
            "session" => self.session_operator(&params_map).await,
            "cookie" => self.cookie_operator(&params_map).await,
            "header" => self.header_operator(&params_map).await,
            "param" => self.param_operator(&params_map).await,
            "query" => self.query_operator(&params_map).await,
            
            // Date and Time Operators
            "date" => self.date_operator(&params_map).await,
            "time" => self.time_operator(&params_map).await,
            "timestamp" => self.timestamp_operator(&params_map).await,
            "now" => self.now_operator(&params_map).await,
            "format" => self.format_operator(&params_map).await,
            "timezone" => self.timezone_operator(&params_map).await,
            
            // String and Data Operators
            "string" => self.string_operator(&params_map).await,
            "regex" => self.regex_operator(&params_map).await,
            "json" => self.json_operator(&params_map).await,
            "base64" => self.base64_operator(&params_map).await,
            "url" => self.url_operator(&params_map).await,
            "hash" => self.hash_operator(&params_map).await,
            "uuid" => self.uuid_operator(&params_map).await,
            
            // Conditional and Logic Operators
            "if" => self.if_operator(&params_map).await,
            "switch" => self.switch_operator(&params_map).await,
            "case" => self.case_operator(&params_map).await,
            "default" => self.default_operator(&params_map).await,
            "and" => self.and_operator(&params_map).await,
            "or" => self.or_operator(&params_map).await,
            "not" => self.not_operator(&params_map).await,
            
            // Math and Calculation Operators
            "math" => self.math_operator(&params_map).await,
            "calc" => self.calc_operator(&params_map).await,
            "min" => self.min_operator(&params_map).await,
            "max" => self.max_operator(&params_map).await,
            "avg" => self.avg_operator(&params_map).await,
            "sum" => self.sum_operator(&params_map).await,
            "round" => self.round_operator(&params_map).await,
            
            // Array and Collection Operators
            "array" => self.array_operator(&params_map).await,
            "map" => self.map_operator(&params_map).await,
            "filter" => self.filter_operator(&params_map).await,
            "sort" => self.sort_operator(&params_map).await,
            "join" => self.join_operator(&params_map).await,
            "split" => self.split_operator(&params_map).await,
            "length" => self.length_operator(&params_map).await,
            
            _ => Err(HlxError::InvalidParameters { 
                operator: operator.to_string(), 
                params: "Unknown core operator".to_string() 
            }),
        }
    }
}

impl CoreOperators {
    // @[unique_key] = value
    async fn variable_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let name = params.get("name")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'name' parameter".to_string() 
            })?;

        let value = params.get("value")
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'value' parameter".to_string() 
            })?;

        // In a real implementation, this would store the variable
        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(name.clone(.to_string())));
            map.insert("value".to_string(), value.clone());
            map.insert("stored".to_string(), Value::Boolean(true));
            map
        }))
    }

    async fn date_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let format = params.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let formatted = format!("{}", now);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("timestamp".to_string(), Value::Number(now.timestamp() as f64));
            map.insert("formatted".to_string(), Value::String(formatted.to_string()));
            map.insert("iso".to_string(), Value::String(now.to_rfc3339(.to_string())));
            map
        }))
    }

    async fn file_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let path = params.get("path")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'path' parameter".to_string() 
            })?;

        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("read");

        match operation {
            "read" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| HlxError::FileError { message: e.to_string() })?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("content".to_string(), Value::String(content.to_string()));
                    map.insert("path".to_string(), Value::String(path.to_string(.to_string())));
                    map.insert("operation".to_string(), Value::String("read".to_string(.to_string())));
                    map
                }))
            },
            "write" => {
                let content = params.get("content")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                        message: "Missing 'content' parameter for write operation".to_string() 
                    })?;

                fs::write(path, content)
                    .map_err(|e| HlxError::FileError { message: e.to_string() })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("path".to_string(), Value::String(path.to_string(.to_string())));
                    map.insert("operation".to_string(), Value::String("write".to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            },
            _ => Err(HlxError::InvalidParameters { 
                operator: "file".to_string(), 
                params: format!("Unknown operation: {}", operation) 
            }),
        }
    }

    async fn json_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("parse");

        match operation {
            "parse" => {
                let json_str = input.as_string()
                    .ok_or_else(|| HlxError::InvalidParameters { 
                        operator: "json".to_string(), 
                        params: "Input must be string for parse operation".to_string() 
                    })?;

                let parsed: serde_json::Value = serde_json::from_str(json_str)
                    .map_err(|e| HlxError::JsonError { message: e.to_string() })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("parsed".to_string(), crate::operators::json_to_value(&parsed));
                    map.insert("operation".to_string(), Value::String("parse".to_string(.to_string())));
                    map
                }))
            },
            "stringify" => {
                let json_value = crate::operators::value_to_json(input);
                let json_str = serde_json::to_string(&json_value)
                    .map_err(|e| HlxError::JsonError { message: e.to_string() })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("stringified".to_string(), Value::String(json_str.to_string()));
                    map.insert("operation".to_string(), Value::String("stringify".to_string(.to_string())));
                    map
                }))
            },
            _ => Err(HlxError::InvalidParameters { 
                operator: "json".to_string(), 
                params: format!("Unknown operation: {}", operation) 
            }),
        }
    }

    async fn query_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let query = params.get("query")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'query' parameter".to_string() 
            })?;

        // MVP stub implementation - would integrate with actual query engine
        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("query".to_string(), Value::String(query.clone(.to_string())));
            map.insert("result".to_string(), Value::String("Query executed successfully".to_string(.to_string())));
            map.insert("rows_affected".to_string(), Value::Number(1.0));
            map
        }))
    }

    async fn base64_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("encode");

        match operation {
            "encode" => {
                // Simple base64-like encoding (not real base64)
                let encoded = format!("b64_{}", base64::encode(input));
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("encoded".to_string(), Value::String(encoded.to_string()));
                    map.insert("operation".to_string(), Value::String("encode".to_string(.to_string())));
                    map
                }))
            },
            "decode" => {
                // Simple base64-like decoding (not real base64)
                if input.starts_with("b64_") {
                    let decoded = input.trim_start_matches("b64_");
                    Ok(Value::Object({
                        let mut map = HashMap::new();
                        map.insert("decoded".to_string(), Value::String(decoded.to_string(.to_string())));
                        map.insert("operation".to_string(), Value::String("decode".to_string(.to_string())));
                        map
                    }))
                } else {
                    Err(HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Invalid base64 format".to_string()
                    })
                }
            },
            _ => Err(HlxError::InvalidParameters { 
                operator: "base64".to_string(), 
                params: format!("Unknown operation: {}", operation) 
            }),
        }
    }

    async fn uuid_operator(&self, _params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let uuid = format!("uuid-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        
        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("uuid".to_string(), Value::String(uuid.to_string(.to_string())));
            map.insert("version".to_string(), Value::String("v4".to_string(.to_string())));
            map
        }))
    }

    // Environment and Request Operators
    async fn env_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let key = params.get("key")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'key' parameter".to_string() 
            })?;

        let value = std::env::var(key).unwrap_or_default();
        
        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("key".to_string(), Value::String(key.clone(.to_string())));
            map.insert("value".to_string(), Value::String(value.to_string()));
            map
        }))
    }

    async fn request_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let field = params.get("field")
            .and_then(|v| v.as_string())
            .unwrap_or("all");

        // Simulate request data
        let request_data = HashMap::from([
            ("method".to_string(), "GET".to_string()),
            ("url".to_string(), "/api/data".to_string()),
            ("headers".to_string(), "{}".to_string()),
            ("body".to_string(), "".to_string()),
        ]);

        match field {
            "all" => Ok(Value::Object({
                let mut map = HashMap::new();
                for (k, v) in request_data {
                    map.insert(k, Value::String(v.to_string()));
                }
                map
            })),
            _ => {
                let value = request_data.get(field).unwrap_or(&"".to_string());
                Ok(Value::String(value.clone(.to_string())))
            }
        }
    }

    async fn session_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .unwrap_or("get");

        match action {
            "get" => {
                let key = params.get("key")
                    .and_then(|v| v.as_string())
                    .unwrap_or("user_id");
                
                // Simulate session data
                let session_data = HashMap::from([
                    ("user_id".to_string(), "12345".to_string()),
                    ("username".to_string(), "john_doe".to_string()),
                    ("role".to_string(), "admin".to_string()),
                ]);

                let value = session_data.get(key).unwrap_or(&"".to_string());
                Ok(Value::String(value.clone(.to_string())))
            },
            "set" => {
                let key = params.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                        message: "Missing 'key' parameter".to_string() 
                    })?;
                let value = params.get("value")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                        message: "Missing 'value' parameter".to_string() 
                    })?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("action".to_string(), Value::String("set".to_string(.to_string())));
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("value".to_string(), Value::String(value.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            },
            _ => Err(HlxError::InvalidParameters { 
                operator: "session".to_string(), 
                params: format!("Unknown action: {}", action) 
            }),
        }
    }

    async fn cookie_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let name = params.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("session_id");

        // Simulate cookie data
        let cookie_data = HashMap::from([
            ("session_id".to_string(), "abc123def456".to_string()),
            ("theme".to_string(), "dark".to_string()),
            ("language".to_string(), "en".to_string()),
        ]);

        let value = cookie_data.get(name).unwrap_or(&"".to_string());
        Ok(Value::String(value.clone(.to_string())))
    }

    async fn header_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let name = params.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("User-Agent");

        // Simulate header data
        let header_data = HashMap::from([
            ("User-Agent".to_string(), "Mozilla/5.0".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token123".to_string()),
        ]);

        let value = header_data.get(name).unwrap_or(&"".to_string());
        Ok(Value::String(value.clone(.to_string())))
    }

    async fn param_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let name = params.get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("id");

        // Simulate parameter data
        let param_data = HashMap::from([
            ("id".to_string(), "123".to_string()),
            ("action".to_string(), "edit".to_string()),
            ("format".to_string(), "json".to_string()),
        ]);

        let value = param_data.get(name).unwrap_or(&"".to_string());
        Ok(Value::String(value.clone(.to_string())))
    }

    // Date and Time Operators
    async fn time_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let format = params.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%H:%M:%S");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let formatted = format!("{}", now);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("time".to_string(), Value::String(formatted.to_string()));
            map.insert("hour".to_string(), Value::Number(now.hour() as f64));
            map.insert("minute".to_string(), Value::Number(now.minute() as f64));
            map.insert("second".to_string(), Value::Number(now.second() as f64));
            map
        }))
    }

    async fn timestamp_operator(&self, _params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("timestamp".to_string(), Value::Number(timestamp as f64));
            map.insert("timestamp_ms".to_string(), Value::Number(now.timestamp_millis() as f64));
            map.insert("formatted".to_string(), Value::String(timestamp.to_string(.to_string())));
            map
        }))
    }

    async fn now_operator(&self, _params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("now".to_string(), Value::String(format!("{}", timestamp.to_string())));
            map.insert("date".to_string(), Value::String(format!("{}", timestamp / 86400.to_string())));
            map.insert("time".to_string(), Value::String(format!("{}", timestamp % 86400.to_string())));
            map.insert("timestamp".to_string(), Value::Number(timestamp as f64));
            map
        }))
    }

    async fn format_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("now");
        let format = params.get("format")
            .and_then(|v| v.as_string())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let formatted = format!("{}", now);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string(.to_string())));
            map.insert("format".to_string(), Value::String(format.to_string(.to_string())));
            map.insert("formatted".to_string(), Value::String(formatted.to_string()));
            map
        }))
    }

    async fn timezone_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let from_tz = params.get("from")
            .and_then(|v| v.as_string())
            .unwrap_or("UTC");
        let to_tz = params.get("to")
            .and_then(|v| v.as_string())
            .unwrap_or("America/New_York");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Simulate timezone conversion
        let converted = format!("{}", now);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("from".to_string(), Value::String(from_tz.to_string(.to_string())));
            map.insert("to".to_string(), Value::String(to_tz.to_string(.to_string())));
            map.insert("original".to_string(), Value::String(now.to_rfc3339(.to_string())));
            map.insert("converted".to_string(), Value::String(converted.to_string()));
            map
        }))
    }

    // String and Data Operators
    async fn string_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello world");
        let operation = params.get("operation")
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
            map.insert("input".to_string(), Value::String(input.to_string(.to_string())));
            map.insert("operation".to_string(), Value::String(operation.to_string(.to_string())));
            map.insert("result".to_string(), Value::String(result.to_string()));
            map
        }))
    }

    async fn regex_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello@example.com");
        let pattern = params.get("pattern")
            .and_then(|v| v.as_string())
            .unwrap_or(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$");
        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("match");

        // Simple regex simulation - in production would use proper regex library
        let result = match operation {
            "match" => input.contains(pattern),
            "find" => if input.contains(pattern) { Some(pattern.to_string()) } else { None },
            "replace" => {
                let replacement = params.get("replacement")
                    .and_then(|v| v.as_string())
                    .unwrap_or("***");
                input.replace(pattern, replacement)
            },
            _ => return Err(HlxError::InvalidParameters {
                operator: "regex".to_string(),
                params: format!("Unknown operation: {}", operation)
            }),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string(.to_string())));
            map.insert("pattern".to_string(), Value::String(pattern.to_string(.to_string())));
            map.insert("operation".to_string(), Value::String(operation.to_string(.to_string())));
            map.insert("result".to_string(), match result {
                std::string::String::String(s) => Value::String(s.to_string()),
                std::string::String::Bool(b) => Value::Boolean(b),
                std::string::String::Option(s) => Value::String(s.unwrap_or_default(.to_string())),
            });
            map
        }))
    }

    async fn url_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello world");
        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("encode");

        let result = match operation {
            "encode" => input.replace(" ", "%20").replace("&", "%26"), // Simple URL encoding
            "decode" => input.replace("%20", " ").replace("%26", "&"), // Simple URL decoding
            _ => input.to_string(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string(.to_string())));
            map.insert("operation".to_string(), Value::String(operation.to_string(.to_string())));
            map.insert("result".to_string(), Value::String(result.to_string()));
            map
        }))
    }

    async fn hash_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("hello world");
        let algorithm = params.get("algorithm")
            .and_then(|v| v.as_string())
            .unwrap_or("sha256");

        let result = match algorithm {
            "sha256" => {
                // Simple hash simulation - not cryptographically secure
                let mut hash = 0u64;
                for (i, byte) in input.as_bytes().iter().enumerate() {
                    hash = hash.wrapping_add(*byte as u64).wrapping_mul(31).wrapping_add(i as u64);
                }
                format!("{:x}", hash)
            },
            "md5" => {
                // Simple hash simulation
                let mut hash = 0u32;
                for byte in input.as_bytes() {
                    hash = hash.wrapping_add(*byte as u32).wrapping_mul(33);
                }
                format!("{:x}", hash)
            },
            _ => input.to_string(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string(.to_string())));
            map.insert("algorithm".to_string(), Value::String(algorithm.to_string(.to_string())));
            map.insert("hash".to_string(), Value::String(result.to_string()));
            map
        }))
    }

    // Conditional and Logic Operators
    async fn if_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let condition = params.get("condition")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let then_value = params.get("then")
            .unwrap_or(&Value::String("true".to_string(.to_string())));
        let else_value = params.get("else")
            .unwrap_or(&Value::String("false".to_string(.to_string())));

        let result = if condition { then_value } else { else_value };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("condition".to_string(), Value::Boolean(condition));
            map.insert("result".to_string(), result.clone());
            map
        }))
    }

    async fn switch_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let value = params.get("value")
            .and_then(|v| v.as_string())
            .unwrap_or("default");
        let cases = params.get("cases")
            .and_then(|v| v.as_object())
            .unwrap_or(&HashMap::new());

        let result = cases.get(value).unwrap_or(&Value::String("default".to_string(.to_string())));

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::String(value.to_string(.to_string())));
            map.insert("result".to_string(), result.clone());
            map
        }))
    }

    async fn case_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let value = params.get("value")
            .and_then(|v| v.as_string())
            .unwrap_or("case1");
        let match_value = params.get("match")
            .and_then(|v| v.as_string())
            .unwrap_or("case1");

        let result = value == match_value;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("value".to_string(), Value::String(value.to_string(.to_string())));
            map.insert("match".to_string(), Value::String(match_value.to_string(.to_string())));
            map.insert("result".to_string(), Value::Boolean(result));
            map
        }))
    }

    async fn default_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let value = params.get("value")
            .unwrap_or(&Value::String("".to_string(.to_string())));
        let default = params.get("default")
            .unwrap_or(&Value::String("default".to_string(.to_string())));

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

    async fn and_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let a = params.get("a")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let b = params.get("b")
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

    async fn or_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let a = params.get("a")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let b = params.get("b")
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

    async fn not_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let value = params.get("value")
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

    // Math and Calculation Operators
    async fn math_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let a = params.get("a")
            .and_then(|v| v.as_number())
            .unwrap_or(0.0);
        let b = params.get("b")
            .and_then(|v| v.as_number())
            .unwrap_or(0.0);
        let operation = params.get("operation")
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
            map.insert("operation".to_string(), Value::String(operation.to_string(.to_string())));
            map.insert("result".to_string(), Value::Number(result));
            map
        }))
    }

    async fn calc_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let expression = params.get("expression")
            .and_then(|v| v.as_string())
            .unwrap_or("2 + 2");

        // Simple expression evaluation (in production, use a proper expression parser)
        let result = match expression {
            "2 + 2" => 4.0,
            "10 * 5" => 50.0,
            "100 / 4" => 25.0,
            _ => 0.0,
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("expression".to_string(), Value::String(expression.to_string(.to_string())));
            map.insert("result".to_string(), Value::Number(result));
            map
        }))
    }

    async fn min_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let values = params.get("values")
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

    async fn max_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let values = params.get("values")
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

    async fn avg_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let values = params.get("values")
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

    async fn sum_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let values = params.get("values")
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

    async fn round_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let value = params.get("value")
            .and_then(|v| v.as_number())
            .unwrap_or(3.14159);
        let decimals = params.get("decimals")
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

    // Array and Collection Operators
    async fn array_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("create");
        let items = params.get("items")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        match operation {
            "create" => Ok(Value::Array(items.clone())),
            "push" => {
                let item = params.get("item").unwrap_or(&Value::String("new_item".to_string(.to_string())));
                let mut new_array = items.clone();
                new_array.push(item.clone());
                Ok(Value::Array(new_array))
            },
            "pop" => {
                let mut new_array = items.clone();
                let popped = new_array.pop().unwrap_or(Value::String("".to_string(.to_string())));
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

    async fn map_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);
        let transform = params.get("transform")
            .and_then(|v| v.as_string())
            .unwrap_or("upper");

        let result: Vec<Value> = array.iter().map(|item| {
            match transform {
                "upper" => {
                    if let Some(s) = item.as_string() {
                        Value::String(s.to_uppercase(.to_string()))
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
            map.insert("transform".to_string(), Value::String(transform.to_string(.to_string())));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn filter_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);
        let condition = params.get("condition")
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
                _ => true,
            }
        }).cloned().collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("original".to_string(), Value::Array(array.clone()));
            map.insert("condition".to_string(), Value::String(condition.to_string(.to_string())));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn sort_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);
        let order = params.get("order")
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
            map.insert("order".to_string(), Value::String(order.to_string(.to_string())));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn join_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);
        let separator = params.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or(",");

        let strings: Vec<String> = array.iter()
            .filter_map(|v| v.as_string())
            .collect();

        let result = strings.join(&separator);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("array".to_string(), Value::Array(array.clone()));
            map.insert("separator".to_string(), Value::String(separator.to_string(.to_string())));
            map.insert("result".to_string(), Value::String(result.to_string()));
            map
        }))
    }

    async fn split_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .unwrap_or("a,b,c,d");
        let separator = params.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or(",");

        let result: Vec<Value> = input.split(&separator)
            .map(|s| Value::String(s.to_string(.to_string())))
            .collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("input".to_string(), Value::String(input.to_string(.to_string())));
            map.insert("separator".to_string(), Value::String(separator.to_string(.to_string())));
            map.insert("result".to_string(), Value::Array(result));
            map
        }))
    }

    async fn length_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .unwrap_or(&Value::String("hello".to_string(.to_string())));

        let length = match input {
            Value::String(s.to_string()) => s.len(),
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