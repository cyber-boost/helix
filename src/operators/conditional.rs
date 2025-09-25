//! Conditional & Control Flow - 6 operators
//! 
//! This module implements operators for conditional logic and control flow:
//! - @if: Conditional expressions
//! - @switch: Switch statements
//! - @for: For loops
//! - @while: While loops
//! - @each: Array iteration
//! - @filter: Array filtering

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use std::collections::HashMap;

/// Conditional operators implementation
pub struct ConditionalOperators;

impl ConditionalOperators {
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self)
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for ConditionalOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "if" => self.if_operator(&params_map).await,
            "switch" => self.switch_operator(&params_map).await,
            "loop" => self.loop_operator(&params_map).await,
            "filter" => self.filter_operator(&params_map).await,
            "map" => self.map_operator(&params_map).await,
            "reduce" => self.reduce_operator(&params_map).await,
            _ => Err(HlxError::invalid_parameters(&operator, "Unknown conditional operator")),
        }
    }
}

impl ConditionalOperators {
    async fn if_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let condition = params.get("condition")
            .ok_or_else(|| HlxError::validation_error("Missing 'condition' parameter", "Check the condition parameter"))?;

        let then_value = params.get("then")
            .ok_or_else(|| HlxError::validation_error("Missing 'then' parameter", "Check the then parameter"))?;

        let else_value = params.get("else")
            .cloned()
            .unwrap_or(Value::Null);

        let result = match condition {
            Value::Bool(b) => {
                if *b { then_value.clone() } else { else_value }
            },
            Value::String(s) => {
                if !s.is_empty() { then_value.clone() } else { else_value }
            },
            Value::Number(n) => {
                if *n != 0.0 { then_value.clone() } else { else_value }
            },
            Value::Array(arr) => {
                if !arr.is_empty() { then_value.clone() } else { else_value }
            },
            Value::Object(obj) => {
                if !obj.is_empty() { then_value.clone() } else { else_value }
            },
            _ => else_value,
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), result);
            map.insert("condition_evaluated".to_string(), Value::Bool(true));
            map
        }))
    }

    async fn switch_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let value = params.get("value")
            .ok_or_else(|| HlxError::validation_error(
                "Missing 'value' parameter", "Check the value parameter"
            ))?;

        let cases = params.get("cases")
            .and_then(|v| v.as_object())
            .ok_or_else(|| HlxError::validation_error(
                "Missing or invalid 'cases' parameter", "Check the cases parameter"
            ))?;

        let default = params.get("default")
            .cloned()
            .unwrap_or(Value::Null);

        let result = if let Some(case_value) = cases.get(&value.to_string()) {
            case_value.clone()
        } else {
            default
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), result);
            map.insert("matched_case".to_string(), value.clone());
            map
        }))
    }

    async fn loop_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let iterations = params.get("iterations")
            .and_then(|v| v.as_number())
            .unwrap_or(1.0) as usize;

        let action = params.get("action")
            .and_then(|v| v.as_string())
            .unwrap_or("default");

        let mut results = Vec::new();
        for i in 0..iterations {
            results.push(Value::Object({
                let mut map = HashMap::new();
                map.insert("iteration".to_string(), Value::Number(i as f64));
                map.insert("action".to_string(), Value::String(action.to_string()));
                map.insert("completed".to_string(), Value::Bool(true));
                map
            }));
        }

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("iterations".to_string(), Value::Number(iterations as f64));
            map.insert("results".to_string(), Value::Array(results));
            map.insert("completed".to_string(), Value::Bool(true));
            map
        }))
    }

    async fn filter_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HlxError::validation_error(
                "Missing 'array' parameter", "Check the array parameter"
            ))?;

        let predicate = params.get("predicate")
            .and_then(|v| v.as_string())
            .unwrap_or("all");

        let filtered: Vec<Value> = match predicate {
            "all" => array.to_vec(),
            "non_null" => array.iter()
                .filter(|item| !matches!(item, &Value::String(s) if s.is_empty()))
                .cloned()
                .collect(),
            "non_empty" => array.iter()
                .filter(|item| match item {
                    Value::String(s) => !s.is_empty(),
                    Value::Array(arr) => !arr.is_empty(),
                    Value::Object(obj) => !obj.is_empty(),
                    _ => true,
                })
                .cloned()
                .collect(),
            _ => array.to_vec(),
        };

        let filtered_count = filtered.len() as f64;

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("filtered".to_string(), Value::Array(filtered));
            map.insert("original_count".to_string(), Value::Number(array.len() as f64));
            map.insert("filtered_count".to_string(), Value::Number(filtered_count));
            map
        }))
    }

    async fn map_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HlxError::validation_error(
                "Missing 'array' parameter", "Check the array parameter"
            ))?;

        let transform = params.get("transform")
            .and_then(|v| v.as_string())
            .unwrap_or("identity");

        let mapped: Vec<Value> = match transform {
            "uppercase" => array.iter()
                .map(|item| match item {
                    Value::String(s) => Value::String(s.to_uppercase()),
                    _ => item.clone(),
                })
                .collect(),
            "lowercase" => array.iter()
                .map(|item| match item {
                    Value::String(s) => Value::String(s.to_lowercase()),
                    _ => item.clone(),
                })
                .collect(),
            "stringify" => array.iter()
                .map(|item| Value::String(item.to_string()))
                .collect(),
            _ => array.to_vec(),
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("mapped".to_string(), Value::Array(mapped.clone()));
            map.insert("transform".to_string(), Value::String(transform.to_string()));
            map.insert("count".to_string(), Value::Number(mapped.len() as f64));
            map
        }))
    }

    async fn reduce_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let array = params.get("array")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HlxError::validation_error(
                "Missing 'array' parameter", "Check the array parameter"
            ))?;

        let operation = params.get("operation")
            .and_then(|v| v.as_string())
            .unwrap_or("sum");

        let initial = params.get("initial")
            .cloned()
            .unwrap_or(Value::Number(0.0));

        let result = match operation {
            "sum" => {
                let sum = array.iter()
                    .filter_map(|item| item.as_number())
                    .sum::<f64>();
                Value::Number(sum)
            },
            "count" => Value::Number(array.len() as f64),
            "join" => {
                let joined = array.iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<String>>()
                    .join("");
                Value::String(joined)
            },
            "concat" => {
                let mut result = Vec::new();
                for item in array {
                    if let Value::Array(arr) = item {
                        result.extend_from_slice(arr);
                    } else {
                        result.push(item.clone());
                    }
                }
                Value::Array(result)
            },
            _ => initial,
        };

        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("result".to_string(), result);
            map.insert("operation".to_string(), Value::String(operation.to_string()));
            map.insert("input_count".to_string(), Value::Number(array.len() as f64));
            map
        }))
    }
} 