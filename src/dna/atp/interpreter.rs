use crate::atp::ast::{HelixAst, Expression, Statement, Declaration};
use crate::hel::error::HlxError;
use crate::ops::engine::OperatorEngine;
use crate::ops::OperatorParser;
use std::collections::HashMap;
pub struct HelixInterpreter {
    operator_engine: OperatorEngine,
    ops_parser: OperatorParser,
    variables: HashMap<String, Value>,
}
pub use crate::atp::types::Value;
pub use crate::atp::ast::{BinaryOperator,LoadDecl};
impl HelixInterpreter {
    pub async fn new() -> Result<Self, HlxError> {
        let operator_engine = OperatorEngine::new().await?;
        let ops_parser = match OperatorParser::new().await {
            Ok(parser) => parser,
            Err(e) => return Err(HlxError::execution_error(
                format!("Failed to create operator parser: {}", e),
                "Check operator configuration"
            )),
        };
        Ok(Self {
            operator_engine,
            ops_parser,
            variables: HashMap::new(),
        })
    }
    pub async fn execute_ast(&mut self, ast: &HelixAst) -> Result<Value, HlxError> {
        let mut result = Value::String("".to_string());
        for declaration in &ast.declarations {
            match declaration {
                Declaration::Section(section) => {
                    result = self.execute_section(&section).await?;
                }
                Declaration::Load(load_decl) => {
                    result = self.execute_load(&load_decl).await?;
                }
                _ => {
                    result = Value::String(
                        format!("Declaration processed: {:?}", declaration),
                    );
                }
            }
        }
        Ok(result)
    }
    async fn execute_section(
        &mut self,
        section: &crate::atp::ast::SectionDecl,
    ) -> Result<Value, HlxError> {
        let mut result = HashMap::new();
        for (k, v) in &section.properties {
            let types_value = v.to_value();
            let converted_value = match types_value {
                Value::String(s) => Value::String(s),
                Value::Number(n) => Value::Number(n),
                Value::Bool(b) => Value::Bool(b),
                Value::Array(arr) => {
                    Value::Array(
                        arr
                            .into_iter()
                            .map(|item| match item {
                                Value::String(s) => Value::String(s),
                                Value::Number(n) => Value::Number(n),
                                Value::Bool(b) => Value::Bool(b),
                                Value::Duration(d) => {
                                    Value::String(format!("{} {:?}", d.value, d.unit))
                                }
                                Value::Reference(r) => Value::String(r),
                                Value::Array(_) => {
                                    Value::String(
                                        "Array conversion not implemented".to_string(),
                                    )
                                }
                                Value::Object(_) => {
                                    Value::String(
                                        "Object conversion not implemented".to_string(),
                                    )
                                }
                                Value::Null => Value::Null,
                                Value::Identifier(i) => Value::String(i),
                            })
                            .collect(),
                    )
                }
                Value::Object(obj) => {
                    Value::Object(
                        obj
                            .into_iter()
                            .map(|(k, v)| (
                                k,
                                match v {
                                    Value::String(s) => Value::String(s),
                                    Value::Number(n) => Value::Number(n),
                                    Value::Bool(b) => Value::Bool(b),
                                    Value::Duration(d) => {
                                        Value::String(format!("{} {:?}", d.value, d.unit))
                                    }
                                    Value::Reference(r) => Value::String(r),
                                    Value::Array(_) => {
                                        Value::String(
                                            "Array conversion not implemented".to_string(),
                                        )
                                    }
                                    Value::Object(_) => {
                                        Value::String(
                                            "Object conversion not implemented".to_string(),
                                        )
                                    }
                                    Value::Null => Value::Null,
                                    Value::Identifier(i) => Value::String(i),
                                },
                            ))
                            .collect(),
                    )
                }
                Value::Duration(d) => {
                    Value::String(format!("{} {:?}", d.value, d.unit))
                }
                Value::Reference(r) => Value::String(r),
                Value::Null => Value::Null,
                Value::Identifier(i) => Value::String(i),
            };
            result.insert(k.clone(), converted_value);
        }
        Ok(Value::Object(result))
    }
    async fn execute_load(
        &mut self,
        load: &LoadDecl,
    ) -> Result<Value, HlxError> {
        Ok(
            Value::Object({
                let mut map = HashMap::new();
                map.insert("loaded".to_string(), Value::String(load.file_name.clone()));
                map.insert("type".to_string(), Value::String("file".to_string()));
                map
            }),
        )
    }
    async fn execute_statement(
        &mut self,
        statement: &Statement,
    ) -> Result<Value, HlxError> {
        match statement {
            Statement::Expression(expr) => self.evaluate_expression(expr).await,
            Statement::Assignment(var_name, expr) => {
                let value = self.evaluate_expression(expr).await?;
                self.variables.insert(var_name.clone(), value.clone());
                Ok(value)
            }
            Statement::Declaration(_) => {
                Ok(Value::String("Declaration executed".to_string()))
            }
        }
    }
    async fn evaluate_expression(
        &mut self,
        expr: &Expression,
    ) -> Result<Value, HlxError> {
        match expr {
            Expression::String(s) => {
                if s.starts_with('@') || s.contains(" + ") || s.contains('?')
                    || s.contains("$")
                {
                    match self.ops_parser.evaluate_expression(expr).await {
                        Ok(value) => Ok(self.convert_ops_value_to_types_value(value)),
                        Err(e) => {
                            Err(
                                HlxError::execution_error(
                                    format!("Operator evaluation failed: {}", e),
                                    "Check operator syntax and parameters",
                                ),
                            )
                        }
                    }
                } else {
                    Ok(Value::String(s.clone()))
                }
            }
            Expression::Number(n) => Ok(Value::Number(*n)),
            Expression::Bool(b) => Ok(Value::Bool(*b)),
            Expression::Null => Ok(Value::Null),
            Expression::Duration(d) => {
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
                    result
                        .insert(
                            key.clone(),
                            Box::pin(self.evaluate_expression(value)).await?,
                        );
                }
                Ok(Value::Object(result))
            }
            Expression::Variable(name) => {
                self.variables
                    .get(name)
                    .cloned()
                    .ok_or_else(|| HlxError::execution_error(
                        format!("Variable '{}' not found", name),
                        "Check variable name and scope",
                    ))
            }
            Expression::OperatorCall(operator, key, sub_key, value) => {
                let mut params = HashMap::new();
                params.insert("key".to_string(), Expression::String(key.clone()));
                if let Some(sk) = sub_key {
                    params.insert("sub_key".to_string(), Expression::String(sk.clone()));
                }
                if let Some(v) = value {
                    params.insert("value".to_string(), Expression::String(v.clone()));
                }
                let json_params = self.params_to_json(&params).await?;
                let value_result = self
                    .operator_engine
                    .execute_operator(&operator, &json_params)
                    .await?;
                Ok(self.convert_ops_value_to_types_value(value_result))
            }
            Expression::AtOperatorCall(_operator, _params) => {
                match self.ops_parser.evaluate_expression(&expr).await {
                    Ok(value) => Ok(self.convert_ops_value_to_types_value(value)),
                    Err(_) => {
                        Err(
                            HlxError::validation_error(
                                "AtOperatorCall failed",
                                "Check operator syntax",
                            ),
                        )
                    }
                }
            }
            Expression::Identifier(name) => {
                if let Some(value) = self.variables.get(name) {
                    Ok(value.clone())
                } else {
                    let params = HashMap::new();
                    let json_params = self.params_to_json(&params).await?;
                    let value_result = self
                        .operator_engine
                        .execute_operator(&name, &json_params)
                        .await?;
                    Ok(self.convert_ops_value_to_types_value(value_result))
                }
            }
            Expression::Reference(name) => self.resolve_reference(name),
            Expression::IndexedReference(file, key) => {
                Box::pin(self.resolve_indexed_reference(file, key)).await
            }
            Expression::Pipeline(stages) => Box::pin(self.execute_pipeline(stages)).await,
            Expression::Block(_statements) => {
                Err(
                    HlxError::validation_error(
                        "Block expressions not supported",
                        "Use statement blocks instead",
                    ),
                )
            }
            Expression::TextBlock(lines) => Ok(Value::String(lines.join("\n"))),
            Expression::BinaryOp(left, op, right) => {
                let left_val = Box::pin(self.evaluate_expression(left)).await?;
                let right_val = Box::pin(self.evaluate_expression(right)).await?;
                let op_str = match op {
                    BinaryOperator::Eq => "==",
                    BinaryOperator::Ne => "!=",
                    BinaryOperator::Lt => "<",
                    BinaryOperator::Le => "<=",
                    BinaryOperator::Gt => ">",
                    BinaryOperator::Ge => ">=",
                    BinaryOperator::And => "&&",
                    BinaryOperator::Or => "||",
                    BinaryOperator::Add => "+",
                    BinaryOperator::Sub => "-",
                    BinaryOperator::Mul => "*",
                    BinaryOperator::Div => "/",
                };
                Ok(Value::String(format!("{:?} {} {:?}", left_val, op_str, right_val)))
            }
        }
    }
    async fn params_to_json(
        &mut self,
        params: &HashMap<String, Expression>,
    ) -> Result<String, HlxError> {
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
                "Check parameter types",
            ))
    }
    fn value_to_json_value(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Number(n) => {
                serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Array(arr) => {
                let values: Vec<serde_json::Value> = arr
                    .iter()
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
            Value::Duration(d) => serde_json::Value::String(format!("{} {:?}", d.value, d.unit)),
            Value::Reference(r) => serde_json::Value::String(format!("@{}", r)),
            Value::Identifier(i) => serde_json::Value::String(i.clone()),
        }
    }
    pub fn operator_engine(&self) -> &OperatorEngine {
        &self.operator_engine
    }
    pub fn operator_engine_mut(&mut self) -> &mut OperatorEngine {
        &mut self.operator_engine
    }
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }
    pub fn list_variables(&self) -> Vec<(String, Value)> {
        self.variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
    fn resolve_reference(&self, name: &str) -> Result<Value, HlxError> {
        if let Some(value) = self.variables.get(name) {
            return Ok(value.clone());
        }
        match self.operator_engine.get_variable(name) {
            Ok(value) => Ok(self.convert_ops_value_to_types_value(value)),
            Err(_) => Err(HlxError::execution_error(
                format!("Variable '{}' not found", name),
                "Check variable name and scope",
            )),
        }
    }
    async fn resolve_indexed_reference(
        &mut self,
        file: &str,
        key: &str,
    ) -> Result<Value, HlxError> {
        let base_value = self.resolve_reference(file)?;
        let keys: Vec<&str> = key.split('.').collect();
        let mut current_value = base_value;
        for key_part in keys {
            match &current_value {
                Value::Object(obj) => {
                    current_value = obj.get(key_part).cloned().unwrap_or(Value::Null);
                }
                Value::Array(arr) => {
                    if let Ok(index) = key_part.parse::<usize>() {
                        current_value = arr.get(index).cloned().unwrap_or(Value::Null);
                    } else {
                        return Err(
                            HlxError::execution_error(
                                format!(
                                    "Invalid array index '{}' in '{}[{}]'", key_part, file, key
                                ),
                                "Array indices must be numeric",
                            ),
                        );
                    }
                }
                _ => {
                    return Err(
                        HlxError::execution_error(
                            format!(
                                "Cannot index into non-object/non-array value for '{}[{}]'",
                                file, key
                            ),
                            "Indexed references require object or array base values",
                        ),
                    );
                }
            }
        }
        Ok(current_value)
    }
    async fn execute_pipeline(&mut self, stages: &[String]) -> Result<Value, HlxError> {
        if stages.is_empty() {
            return Err(
                HlxError::execution_error(
                    "Empty pipeline",
                    "Pipelines must contain at least one stage",
                ),
            );
        }
        let mut result = Value::Null;
        for (i, stage) in stages.iter().enumerate() {
            match self.operator_engine.execute_operator(stage, "{}").await {
                Ok(stage_result) => {
                    result = self.convert_ops_value_to_types_value(stage_result);
                }
                Err(e) => {
                    return Err(
                        HlxError::execution_error(
                            format!("Pipeline stage {} failed: {}", i + 1, e),
                            "Check pipeline stage syntax and parameters",
                        ),
                    );
                }
            }
        }
        Ok(result)
    }
    fn convert_ops_value_to_types_value(&self, ops_value: crate::atp::value::Value) -> Value {
        match ops_value {
            crate::atp::value::Value::String(s) => Value::String(s),
            crate::atp::value::Value::Number(n) => Value::Number(n),
            crate::atp::value::Value::Bool(b) => Value::Bool(b),
            crate::atp::value::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(|v| self.convert_ops_value_to_types_value(v)).collect())
            }
            crate::atp::value::Value::Object(obj) => {
                Value::Object(obj.into_iter().map(|(k, v)| (k, self.convert_ops_value_to_types_value(v))).collect())
            }
            crate::atp::value::Value::Null => Value::Null,
            crate::atp::value::Value::Duration(d) => Value::Duration(d),
            crate::atp::value::Value::Reference(r) => Value::Reference(r),
            crate::atp::value::Value::Identifier(i) => Value::Identifier(i),
        }
    }
}
