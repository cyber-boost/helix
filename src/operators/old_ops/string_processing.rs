//! String & Data Processing - 8 operators
//! 
//! This module implements operators for string manipulation and data processing:
//! - @string: String manipulation
//! - @regex: Regular expressions
//! - @hash: Hashing functions
//! - @base64: Base64 encoding
//! - @xml: XML parsing
//! - @yaml: YAML parsing
//! - @csv: CSV processing
//! - @template: Template engine

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use md5;
use quick_xml::Reader;
use serde_json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// String processing operators implementation
pub struct StringOperators;

impl StringOperators {
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self)
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for StringOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "concat" => self.concat_operator(&params_map).await,
            "split" => self.split_operator(&params_map).await,
            "replace" => self.replace_operator(&params_map).await,
            "trim" => self.trim_operator(&params_map).await,
            "upper" => self.upper_operator(&params_map).await,
            "lower" => self.lower_operator(&params_map).await,
            "hash" => self.hash_operator(&params_map).await,
            "format" => self.format_operator(&params_map).await,
            _ => Err(HlxError::InvalidParameters { 
                operator: operator.to_string(), 
                params: "Unknown string operator".to_string() 
            }),
        }
    }
}

impl StringOperators {
    async fn concat_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let strings = params.get("strings")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'strings' parameter".to_string() 
            })?;

        let separator = params.get("separator")
            .and_then(|v| v.as_string())
            .unwrap_or("");

        let concatenated = strings.iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(separator);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(concatenated.to_string()));
            map.insert("count".to_string(), Value::Number(strings.len() as f64));
            map.insert("separator".to_string(), Value::String(separator.to_string(.to_string())));
            map
        }))
    }

    async fn split_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let delimiter = params.get("delimiter")
            .and_then(|v| v.as_string())
            .unwrap_or(" ");

        let parts: Vec<Value> = input.split(delimiter)
            .map(|s| Value::String(s.to_string(.to_string())))
            .collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("parts".to_string(), Value::Array(parts.clone()));
            map.insert("count".to_string(), Value::Number(parts.len() as f64));
            map.insert("delimiter".to_string(), Value::String(delimiter.to_string(.to_string())));
            map
        }))
    }

    async fn replace_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let from = params.get("from")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'from' parameter".to_string() 
            })?;

        let to = params.get("to")
            .and_then(|v| v.as_string())
            .unwrap_or("");

        let replaced = input.replace(from, &to);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(replaced.to_string()));
            map.insert("original".to_string(), Value::String(input.clone(.to_string())));
            map.insert("from".to_string(), Value::String(from.clone(.to_string())));
            map.insert("to".to_string(), Value::String(to.clone(.to_string())));
            map
        }))
    }

    async fn trim_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let mode = params.get("mode")
            .and_then(|v| v.as_string())
            .unwrap_or("both");

        let trimmed = match mode {
            "left" => input.trim_start(),
            "right" => input.trim_end(),
            "both" => input.trim(),
            _ => input.trim(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(trimmed.to_string(.to_string())));
            map.insert("original".to_string(), Value::String(input.clone(.to_string())));
            map.insert("mode".to_string(), Value::String(mode.to_string(.to_string())));
            map
        }))
    }

    async fn upper_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let uppercased = input.to_uppercase();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(uppercased.to_string()));
            map.insert("original".to_string(), Value::String(input.clone(.to_string())));
            map.insert("operation".to_string(), Value::String("uppercase".to_string(.to_string())));
            map
        }))
    }

    async fn lower_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let lowercased = input.to_lowercase();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(lowercased.to_string()));
            map.insert("original".to_string(), Value::String(input.clone(.to_string())));
            map.insert("operation".to_string(), Value::String("lowercase".to_string(.to_string())));
            map
        }))
    }

    async fn hash_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'input' parameter".to_string() 
            })?;

        let algorithm = params.get("algorithm")
            .and_then(|v| v.as_string())
            .unwrap_or("sha256");

        let hash = match algorithm {
            "sha256" => {
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                general_purpose::STANDARD.encode(hasher.finalize())
            },
            "md5" => {
                let mut hasher = md5::Context::new();
                hasher.update(input.as_bytes());
                general_purpose::STANDARD.encode(hasher.finalize())
            },
            _ => return Err(HlxError::HashError { 
                message: format!("Unsupported algorithm: {}", algorithm) 
            }),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("hash".to_string(), Value::String(hash.to_string()));
            map.insert("algorithm".to_string(), Value::String(algorithm.to_string(.to_string())));
            map.insert("input".to_string(), Value::String(input.clone(.to_string())));
            map
        }))
    }

    async fn format_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let template = params.get("template")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None, 
                message: "Missing 'template' parameter".to_string() 
            })?;

        let variables = params.get("variables")
            .and_then(|v| v.as_object())
            .unwrap_or(&HashMap::new());

        let mut result = template.clone();
        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, &value.to_string());
        }

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(result.to_string()));
            map.insert("template".to_string(), Value::String(template.clone(.to_string())));
            map.insert("variables_used".to_string(), Value::Number(variables.len() as f64));
            map
        }))
    }
} 