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
            _ => Err(HlxError::invalid_parameters(operator, "Unknown string operator")),
        }
    }
}

impl StringOperators {
    async fn concat_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let strings = params.get("strings")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HlxError::validation_error("Missing 'strings' parameter", "Check the strings parameter"))?;

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
            map.insert("separator".to_string(), Value::String(separator.to_string()));
            map
        }))
    }

    async fn split_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'input' parameter", "Check the input parameter"))?;

        let delimiter = params.get("delimiter")
            .and_then(|v| v.as_string())
            .unwrap_or(" ");

        let parts: Vec<Value> = input.split(delimiter)
            .map(|s| Value::String(s.to_string()))
            .collect();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("parts".to_string(), Value::Array(parts.clone()));
            map.insert("count".to_string(), Value::Number(parts.len() as f64));
            map.insert("delimiter".to_string(), Value::String(delimiter.to_string()));
            map
        }))
    }

    async fn replace_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'input' parameter", "Check the input parameter"))?;

        let from = params.get("from")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'from' parameter", "Check the from parameter"))?;

        let to = params.get("to")
            .and_then(|v| v.as_string())
            .unwrap_or("");

        let replaced = input.replace(from, &to);

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(replaced.to_string()));
            map.insert("original".to_string(), Value::String(input.to_string()));
            map.insert("from".to_string(), Value::String(from.to_string()));
            map.insert("to".to_string(), Value::String(to.to_string()));
            map
        }))
    }

    async fn trim_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'input' parameter", "Check the input parameter"))?;

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
            map.insert("result".to_string(), Value::String(trimmed.to_string()));
            map.insert("original".to_string(), Value::String(input.to_string()));
            map.insert("mode".to_string(), Value::String(mode.to_string()));
            map
        }))
    }

    async fn upper_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'input' parameter", "Check the input parameter"))?;

        let uppercased = input.to_uppercase();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(uppercased.to_string()));
            map.insert("original".to_string(), Value::String(input.to_string()));
            map.insert("operation".to_string(), Value::String("uppercase".to_string()));
            map
        }))
    }

    async fn lower_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'input' parameter", "Check the input parameter"))?;

        let lowercased = input.to_lowercase();

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(lowercased.to_string()));
            map.insert("original".to_string(), Value::String(input.to_string()));
            map.insert("operation".to_string(), Value::String("lowercase".to_string()));
            map
        }))
    }

    async fn hash_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let input = params.get("input")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'input' parameter", "Check the input parameter"))?;

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
                let digest = md5::compute(input.as_bytes());
                general_purpose::STANDARD.encode(&digest[..])
            },
            _ => return Err(HlxError::hash_error(format!("Unsupported algorithm: {}", algorithm), "Check the hash algorithm parameter")),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("hash".to_string(), Value::String(hash.to_string()));
            map.insert("algorithm".to_string(), Value::String(algorithm.to_string()));
            map.insert("input".to_string(), Value::String(input.to_string()));
            map
        }))
    }

    async fn format_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let template = params.get("template")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::validation_error("Missing 'template' parameter", "Check the template parameter"))?;

        let variables = params.get("variables")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let mut result = template.to_string();
        for (key, value) in &variables {
            let placeholder = format!("${{{}}}", key);
            let value_str = value.to_string();
            result = result.replace(&placeholder, &value_str);
        }

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), Value::String(result.to_string()));
            map.insert("template".to_string(), Value::String(template.to_string()));
            map.insert("variables_used".to_string(), Value::Number(variables.len() as f64));
            map
        }))
    }
} 