//! Helix Language Interpreter
//!
//! This module provides execution capabilities for .hlx files by interpreting
//! the AST and executing operator calls through the operators system.

use crate::ast::{HelixAst, Expression, Statement, Declaration};
use crate::value::Value;
use crate::error::HlxError;
use crate::operators::OperatorEngine;
use crate::ops::OperatorParser;
use std::collections::HashMap;

/// Helix Language Interpreter
pub struct HelixInterpreter {
    operator_engine: OperatorEngine,
    ops_parser: OperatorParser,
    variables: HashMap<String, Value>,
}

impl HelixInterpreter {
    /// Create a new interpreter instance
    pub async fn new() -> Result<Self, HlxError> {
        let operator_engine = OperatorEngine::new().await?;
        let ops_parser = OperatorParser::new().await;

        Ok(Self {
            operator_engine,
            ops_parser,
            variables: HashMap::new(),
        })
    }

    /// Execute a parsed AST
    pub async fn execute_ast(&mut self, ast: &HelixAst) -> Result<Value, HlxError> {
        let mut result = Value::String("".to_string());

        for declaration in &ast.declarations {
            match declaration {
                Declaration::Section(section) => {
                    // Execute section as a block of statements
                    result = self.execute_section(&section).await?;
                }
                Declaration::Load(load_decl) => {
                    // Handle load declarations
                    result = self.execute_load(&load_decl).await?;
                }
                _ => {
                    // For other declarations, just store them
                    result = Value::String(format!("Declaration processed: {:?}", declaration));
                }
            }
        }

        Ok(result)
    }

    /// Execute a section declaration
    async fn execute_section(&mut self, section: &crate::ast::SectionDecl) -> Result<Value, HlxError> {
        // For now, just return the section properties as a value
        let mut result = HashMap::new();
        for (k, v) in &section.properties {
            let types_value = v.to_value();
            let converted_value = match types_value {
                crate::types::Value::String(s) => Value::String(s),
                crate::types::Value::Number(n) => Value::Number(n),
                crate::types::Value::Bool(b) => Value::Bool(b),
                crate::types::Value::Array(arr) => Value::Array(
                    arr.into_iter().map(|item| match item {
                        crate::types::Value::String(s) => Value::String(s),
                        crate::types::Value::Number(n) => Value::Number(n),
                        crate::types::Value::Bool(b) => Value::Bool(b),
                        crate::types::Value::Duration(d) => Value::String(format!("{} {:?}", d.value, d.unit)),
                        crate::types::Value::Reference(r) => Value::String(r),
                        crate::types::Value::Array(_) => Value::String("Array conversion not implemented".to_string()),
                        crate::types::Value::Object(_) => Value::String("Object conversion not implemented".to_string()),
                        crate::types::Value::Null => Value::Null,
                        crate::types::Value::Identifier(i) => Value::String(i),
                    }).collect()
                ),
                crate::types::Value::Object(obj) => Value::Object(
                    obj.into_iter().map(|(k, v)| (k, match v {
                        crate::types::Value::String(s) => Value::String(s),
                        crate::types::Value::Number(n) => Value::Number(n),
                        crate::types::Value::Bool(b) => Value::Bool(b),
                        crate::types::Value::Duration(d) => Value::String(format!("{} {:?}", d.value, d.unit)),
                        crate::types::Value::Reference(r) => Value::String(r),
                        crate::types::Value::Array(_) => Value::String("Array conversion not implemented".to_string()),
                        crate::types::Value::Object(_) => Value::String("Object conversion not implemented".to_string()),
                        crate::types::Value::Null => Value::Null,
                        crate::types::Value::Identifier(i) => Value::String(i),
                    })).collect()
                ),
                crate::types::Value::Duration(d) => Value::String(format!("{} {:?}", d.value, d.unit)),
                crate::types::Value::Reference(r) => Value::String(r),
                crate::types::Value::Null => Value::Null,
                crate::types::Value::Identifier(i) => Value::String(i),
            };
            result.insert(k.clone(), converted_value);
        }
        Ok(Value::Object(result))
    }

    /// Execute a load declaration
    async fn execute_load(&mut self, load: &crate::ast::LoadDecl) -> Result<Value, HlxError> {
        // For now, just return success
        Ok(Value::Object({
            let mut map = HashMap::new();
            map.insert("loaded".to_string(), Value::String(load.file_name.clone()));
            map.insert("type".to_string(), Value::String("file".to_string()));
            map
        }))
    }

    /// Execute a single statement
    async fn execute_statement(&mut self, statement: &Statement) -> Result<Value, HlxError> {
        match statement {
            Statement::Expression(expr) => self.evaluate_expression(expr).await,
            Statement::Assignment(var_name, expr) => {
                let value = self.evaluate_expression(expr).await?;
                self.variables.insert(var_name.clone(), value.clone());
                Ok(value)
            }
            Statement::Declaration(_) => {
                // Declaration handling - for now just return success
                Ok(Value::String("Declaration executed".to_string()))
            }
        }
    }

    /// Evaluate an expression
    async fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, HlxError> {
        match expr {
            Expression::String(s) => {
                // Check if string contains special operators that need ops.rs processing
                if s.starts_with('@') || s.contains(" + ") || s.contains('?') || s.contains("$") {
                    // Use ops parser for special operator evaluation
                    match self.ops_parser.evaluate_expression(expr).await {
                        Ok(value) => Ok(value),
                        Err(e) => Err(HlxError::execution_error(
                            format!("Operator evaluation failed: {}", e),
                            "Check operator syntax and parameters"
                        )),
                    }
                } else {
                    Ok(Value::String(s.clone()))
                }
            },
            Expression::Number(n) => Ok(Value::Number(*n)),
            Expression::Bool(b) => Ok(Value::Bool(*b)),
            Expression::Null => Ok(Value::Null),
            Expression::Duration(d) => {
                // Convert duration to a string representation like elsewhere in the codebase
                Ok(Value::String(format!("{} {:?}", d.value, d.unit)))
            }
            Expression::Array(arr) => {
                let mut values = Vec::new();
                for item in arr {
                    values.push(Box::pin(self.evaluate_expression(item)).await?);
                }
                Ok(Value::Array(values))
            }
            Expression::Object(obj) => {
                let mut result = HashMap::new();
                for (key, value) in obj {
                    result.insert(key.clone(), Box::pin(self.evaluate_expression(value)).await?);
                }
                Ok(Value::Object(result))
            }
            Expression::Variable(name) => {
                // Look up variable
                self.variables.get(name)
                    .cloned()
                    .ok_or_else(|| HlxError::execution_error(
                        format!("Variable '{}' not found", name),
                        "Check variable name and scope"
                    ))
            }
            Expression::OperatorCall(operator, params) => {
                // Convert expression parameters to JSON string for operator execution
                let json_params = self.params_to_json(params).await?;
                let value_result = self.operator_engine.execute_operator(&operator, &json_params).await?;
                Ok(value_result)
            }
            Expression::AtOperatorCall(operator, params) => {
                // For now, delegate to ops parser
                match self.ops_parser.evaluate_expression(&expr).await {
                    Ok(value) => Ok(value),
                    Err(_) => Err(HlxError::validation_error("AtOperatorCall failed", "Check operator syntax")),
                }
            }
            Expression::Identifier(name) => {
                // Try to resolve as variable first, then as operator
                if let Some(value) = self.variables.get(name) {
                    Ok(value.clone())
                } else {
                    // Could be an operator call with no parameters
                    let params = HashMap::new();
                    let json_params = self.params_to_json(&params).await?;
                    let value_result = self.operator_engine.execute_operator(&name, &json_params).await?;
                    Ok(value_result)
                }
            }
            Expression::Reference(name) => {
                // Resolve memory reference from global memory system
                self.resolve_reference(name)
            }
            Expression::IndexedReference(file, key) => {
                // Resolve indexed reference like @file[key] with nested access
                Box::pin(self.resolve_indexed_reference(file, key)).await
            }
            Expression::Pipeline(stages) => {
                // Execute pipeline stages sequentially
                Box::pin(self.execute_pipeline(stages)).await
            }
            Expression::Block(statements) => {
                Err(HlxError::validation_error("Block expressions not supported", "Use statement blocks instead"))
            }
            Expression::TextBlock(lines) => {
                Ok(Value::String(lines.join("\n")))
            }
            Expression::BinaryOp(left, op, right) => {
                // For now, stringify binary operations
                let left_val = Box::pin(self.evaluate_expression(left)).await?;
                let right_val = Box::pin(self.evaluate_expression(right)).await?;
                let op_str = match op {
                    crate::ast::BinaryOperator::Eq => "==",
                    crate::ast::BinaryOperator::Ne => "!=",
                    crate::ast::BinaryOperator::Lt => "<",
                    crate::ast::BinaryOperator::Le => "<=",
                    crate::ast::BinaryOperator::Gt => ">",
                    crate::ast::BinaryOperator::Ge => ">=",
                    crate::ast::BinaryOperator::And => "&&",
                    crate::ast::BinaryOperator::Or => "||",
                    crate::ast::BinaryOperator::Add => "+",
                    crate::ast::BinaryOperator::Sub => "-",
                    crate::ast::BinaryOperator::Mul => "*",
                    crate::ast::BinaryOperator::Div => "/",
                };
                Ok(Value::String(format!("{:?} {} {:?}", left_val, op_str, right_val)))
            }
        }
    }

    /// Convert expression parameters to JSON string for operator execution
    async fn params_to_json(&mut self, params: &HashMap<String, Expression>) -> Result<String, HlxError> {
        let mut json_map = serde_json::Map::new();

        for (key, expr) in params {
            let value = Box::pin(self.evaluate_expression(expr)).await?;
            let json_value = self.value_to_json_value(&value);
            json_map.insert(key.clone(), json_value);
        }

        let json_obj = serde_json::Value::Object(json_map);
        serde_json::to_string(&json_obj)
            .map_err(|e| HlxError::execution_error(
                format!("Failed to serialize parameters: {}", e),
                "Check parameter types"
            ))
    }

    /// Convert our Value type to serde_json::Value
    fn value_to_json_value(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Number(n) => serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Array(arr) => {
                let values: Vec<serde_json::Value> = arr.iter()
                    .map(|v| self.value_to_json_value(v))
                    .collect();
                serde_json::Value::Array(values)
            }
            Value::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.value_to_json_value(v));
                }
                serde_json::Value::Object(map)
            }
            Value::Null => serde_json::Value::Null,
        }
    }

    /// Get access to the underlying operator engine
    pub fn operator_engine(&self) -> &OperatorEngine {
        &self.operator_engine
    }

    /// Get access to the underlying operator engine (mutable)
    pub fn operator_engine_mut(&mut self) -> &mut OperatorEngine {
        &mut self.operator_engine
    }

    /// Set a variable in the interpreter context
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Get a variable from the interpreter context
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// List all variables in the interpreter context
    pub fn list_variables(&self) -> Vec<(String, Value)> {
        self.variables.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Resolve a memory reference from the global memory system
    fn resolve_reference(&self, name: &str) -> Result<Value, HlxError> {
        // First check local variables
        if let Some(value) = self.variables.get(name) {
            return Ok(value.clone());
        }

        // Then check global memory system
        self.operator_engine.get_variable(name)
    }

    /// Resolve an indexed reference like @file[key] with nested access support
    async fn resolve_indexed_reference(&mut self, file: &str, key: &str) -> Result<Value, HlxError> {
        // First get the base value from memory
        let base_value = self.resolve_reference(file)?;

        // Parse the key for nested access (support dot notation)
        let keys: Vec<&str> = key.split('.').collect();
        let mut current_value = base_value;

        for key_part in keys {
            match &current_value {
                Value::Object(obj) => {
                    current_value = obj.get(key_part)
                        .cloned()
                        .unwrap_or(Value::Null);
                }
                Value::Array(arr) => {
                    // Try to parse key as array index
                    if let Ok(index) = key_part.parse::<usize>() {
                        current_value = arr.get(index)
                            .cloned()
                            .unwrap_or(Value::Null);
                    } else {
                        return Err(HlxError::execution_error(
                            format!("Invalid array index '{}' in '{}[{}]'", key_part, file, key),
                            "Array indices must be numeric"
                        ));
                    }
                }
                _ => {
                    return Err(HlxError::execution_error(
                        format!("Cannot index into non-object/non-array value for '{}[{}]'", file, key),
                        "Indexed references require object or array base values"
                    ));
                }
            }
        }

        Ok(current_value)
    }

    /// Execute a pipeline of operations sequentially
    async fn execute_pipeline(&mut self, stages: &[String]) -> Result<Value, HlxError> {
        if stages.is_empty() {
            return Err(HlxError::execution_error(
                "Empty pipeline",
                "Pipelines must contain at least one stage"
            ));
        }

        let mut result = Value::Null;

        for (i, stage) in stages.iter().enumerate() {
            // For now, treat each stage as an operator call
            // In a more sophisticated implementation, this could parse stage syntax
            match self.operator_engine.execute_operator(stage, "{}").await {
                Ok(stage_result) => {
                    result = stage_result;
                }
                Err(e) => {
                    return Err(HlxError::execution_error(
                        format!("Pipeline stage {} failed: {}", i + 1, e),
                        "Check pipeline stage syntax and parameters"
                    ));
                }
            }
        }

        Ok(result)
    }
}