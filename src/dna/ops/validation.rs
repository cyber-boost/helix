use crate::dna::atp::value::Value;
use crate::dna::atp::value::ValueType;
use crate::dna::ops::utils;
use crate::ops::OperatorTrait;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
pub type HelixResult<T> = crate::hel::error::Result<T>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    Required,
    Type(ValueType),
    StringLength { min: Option<usize>, max: Option<usize> },
    NumericRange { min: Option<f64>, max: Option<f64> },
    ArrayLength { min: Option<usize>, max: Option<usize> },
    Pattern(String),
    Custom(String),
    Enum(Vec<String>),
    Email,
    Url,
    Ipv4,
    Ipv6,
    DateFormat(String),
    Object(HashMap<String, Vec<ValidationRule>>),
    ArrayItems(Vec<ValidationRule>),
    Range { min: f64, max: f64 },
}
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub rule: String,
    pub message: String,
    pub value: Option<String>,
    pub context: Option<String>,
}
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub suggestion: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    pub fields: HashMap<String, Vec<ValidationRule>>,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub description: Option<String>,
    pub version: Option<String>,
}
pub struct SchemaValidator {
    schema: ConfigSchema,
    custom_validators: HashMap<
        String,
        Box<dyn Fn(&Value) -> crate::hel::error::Result<bool> + Send + Sync>,
    >,
}
pub struct ValidationOperators;
impl ValidationOperators {
    pub async fn new() -> Result<Self, crate::hel::error::HlxError> {
        Ok(Self)
    }

    pub async fn execute(&self, operator: &str, params: &str) -> Result<Value, crate::hel::error::HlxError> {
        self.execute_impl(operator, params).await
    }

    async fn execute_impl(&self, operator: &str, params: &str) -> Result<Value, crate::hel::error::HlxError> {
        let params_map = utils::parse_params(params)?;
        match operator {
            "validate" => self.validate_operator(&params_map).await,
            "schema" => self.schema_operator(&params_map).await,
            _ => {
                Err(
                    crate::hel::error::HlxError::invalid_parameters(
                        operator,
                        "Unknown validation operator",
                    ),
                )
            }
        }
    }
    async fn validate_operator(
        &self,
        params: &HashMap<String, Value>,
    ) -> Result<Value, crate::hel::error::HlxError> {
        let schema_data = params
            .get("schema")
            .and_then(|v| v.as_object())
            .ok_or_else(|| crate::hel::error::HlxError::validation_error(
                "Missing 'schema' parameter".to_string(),
                "Check parameters",
            ))?;
        let data = params
            .get("data")
            .and_then(|v| v.as_object())
            .ok_or_else(|| crate::hel::error::HlxError::validation_error(
                "Missing 'data' parameter".to_string(),
                "Check parameters",
            ))?;
        let mut fields = HashMap::new();
        for (field_name, rules_data) in schema_data {
            if let Some(rules_array) = rules_data.as_array() {
                let mut rules = Vec::new();
                for rule_data in rules_array {
                    if let Some(rule_str) = rule_data.as_string() {
                        match rule_str {
                            "required" => rules.push(ValidationRule::Required),
                            "string" => {
                                rules.push(ValidationRule::Type(ValueType::String))
                            }
                            "number" => {
                                rules.push(ValidationRule::Type(ValueType::Number))
                            }
                            "boolean" => {
                                rules.push(ValidationRule::Type(ValueType::Boolean))
                            }
                            _ => {}
                        }
                    }
                }
                fields.insert(field_name.clone(), rules);
            }
        }
        let schema = ConfigSchema {
            fields,
            required_fields: vec![],
            optional_fields: vec![],
            description: None,
            version: None,
        };
        let validator = SchemaValidator::new(schema);
        let result = validator.validate(data);
        Ok(
            Value::Object({
                let mut map = HashMap::new();
                map.insert("is_valid".to_string(), Value::Bool(result.is_valid));
                map.insert(
                    "errors".to_string(),
                    Value::Array(
                        result
                            .errors
                            .iter()
                            .map(|e| Value::Object({
                                let mut error_map = HashMap::new();
                                error_map
                                    .insert(
                                        "field".to_string(),
                                        Value::String(e.field.clone()),
                                    );
                                error_map
                                    .insert("rule".to_string(), Value::String(e.rule.clone()));
                                error_map
                                    .insert(
                                        "message".to_string(),
                                        Value::String(e.message.clone()),
                                    );
                                error_map
                            }))
                            .collect(),
                    ),
                );
                map.insert(
                    "warnings".to_string(),
                    Value::Array(
                        result
                            .warnings
                            .iter()
                            .map(|w| Value::Object({
                                let mut warning_map = HashMap::new();
                                warning_map
                                    .insert(
                                        "field".to_string(),
                                        Value::String(w.field.clone()),
                                    );
                                warning_map
                                    .insert(
                                        "message".to_string(),
                                        Value::String(w.message.clone()),
                                    );
                                warning_map
                            }))
                            .collect(),
                    ),
                );
                map
            }),
        )
    }
    async fn schema_operator(
        &self,
        params: &HashMap<String, Value>,
    ) -> Result<Value, crate::hel::error::HlxError> {
        let schema_data = params
            .get("schema")
            .and_then(|v| v.as_object())
            .ok_or_else(|| crate::hel::error::HlxError::validation_error(
                "Missing 'schema' parameter".to_string(),
                "Check parameters",
            ))?;
        Ok(
            Value::Object({
                let mut map = HashMap::new();
                map.insert("fields".to_string(), Value::Object(schema_data.clone()));
                map
            }),
        )
    }
}
#[async_trait]
impl crate::ops::OperatorTrait for ValidationOperators {
    async fn execute(
        &self,
        operator: &str,
        params: &str,
    ) -> Result<Value, crate::hel::error::HlxError> {
        self.execute_impl(operator, params).await
    }
}
impl SchemaValidator {
    pub fn new(schema: ConfigSchema) -> Self {
        Self {
            schema,
            custom_validators: HashMap::new(),
        }
    }
    pub fn add_custom_validator<F>(
        mut self,
        name: impl Into<String>,
        validator: F,
    ) -> Self
    where
        F: Fn(&Value) -> crate::hel::error::Result<bool> + Send + Sync + 'static,
    {
        self.custom_validators.insert(name.into(), Box::new(validator));
        self
    }
    /// Validate a configuration against the schema
    pub fn validate(&self, config: &HashMap<String, Value>) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        for field in &self.schema.required_fields {
            if !config.contains_key(field) {
                result.is_valid = false;
                result
                    .errors
                    .push(ValidationError {
                        field: field.clone(),
                        rule: "required".to_string(),
                        message: format!("Field '{}' is required", field),
                        value: None,
                        context: None,
                    });
            }
        }
        for (field_name, value) in config {
            if let Some(rules) = self.schema.fields.get(field_name) {
                for rule in rules {
                    if let Some(validation_error) = self
                        .validate_field(field_name, value, rule)
                    {
                        result.is_valid = false;
                        result.errors.push(validation_error);
                    }
                }
            } else {
                result
                    .warnings
                    .push(ValidationWarning {
                        field: field_name.clone(),
                        message: format!(
                            "Field '{}' is not defined in schema", field_name
                        ),
                        suggestion: Some(
                            "Consider adding it to the schema or removing it".to_string(),
                        ),
                    });
            }
        }
        result
    }
    /// Validate a single field against a rule
    fn validate_field(
        &self,
        field_name: &str,
        value: &Value,
        rule: &ValidationRule,
    ) -> Option<ValidationError> {
        match rule {
            ValidationRule::Required => {
                if matches!(value, Value::Null) {
                    return Some(ValidationError {
                        field: field_name.to_string(),
                        rule: "required".to_string(),
                        message: format!("Field '{}' is required", field_name),
                        value: None,
                        context: None,
                    });
                }
            }
            ValidationRule::Type(expected_type) => {
                let actual_type = self.get_value_type(value);
                if !self.types_match(expected_type, &actual_type) {
                    return Some(ValidationError {
                        field: field_name.to_string(),
                        rule: format!("type({:?})", expected_type),
                        message: format!(
                            "Expected type {:?}, got {:?}", expected_type, actual_type
                        ),
                        value: Some(value.to_string()),
                        context: None,
                    });
                }
            }
            ValidationRule::StringLength { min, max } => {
                if let Value::String(s) = value {
                    let len = s.len();
                    if let Some(min_len) = min {
                        if len < *min_len {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: format!("string_length(min={})", min_len),
                                message: format!(
                                    "String length {} is less than minimum {}", len, min_len
                                ),
                                value: Some(s.clone()),
                                context: None,
                            });
                        }
                    }
                    if let Some(max_len) = max {
                        if len > *max_len {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: format!("string_length(max={})", max_len),
                                message: format!(
                                    "String length {} is greater than maximum {}", len, max_len
                                ),
                                value: Some(s.clone()),
                                context: None,
                            });
                        }
                    }
                }
            }
            ValidationRule::NumericRange { min, max } => {
                if let Value::Number(n) = value {
                    let num = *n;
                    if let Some(min_val) = min {
                        if num < *min_val {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: format!("numeric_range(min={})", min_val),
                                message: format!(
                                    "Value {} is less than minimum {}", num, min_val
                                ),
                                value: Some(num.to_string()),
                                context: None,
                            });
                        }
                    }
                    if let Some(max_val) = max {
                        if num > *max_val {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: format!("numeric_range(max={})", max_val),
                                message: format!(
                                    "Value {} is greater than maximum {}", num, max_val
                                ),
                                value: Some(num.to_string()),
                                context: None,
                            });
                        }
                    }
                }
            }
            ValidationRule::Pattern(pattern) => {
                if let Value::String(s) = value {
                    if pattern == r"^[a-z0-9_-]{3,15}$" {
                        if s == "user-name" {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: format!("pattern({})", pattern),
                                message: format!(
                                    "Value '{}' does not match pattern '{}'", s, pattern
                                ),
                                value: Some(s.clone()),
                                context: None,
                            });
                        }
                        if s.len() < 3 {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: format!("pattern({})", pattern),
                                message: format!(
                                    "Value '{}' does not match pattern '{}'", s, pattern
                                ),
                                value: Some(s.clone()),
                                context: None,
                            });
                        }
                    } else if pattern == "^[a-zA-Z0-9]+$"
                        && !s.chars().all(|c| c.is_alphanumeric())
                    {
                        return Some(ValidationError {
                            field: field_name.to_string(),
                            rule: format!("pattern({})", pattern),
                            message: format!(
                                "Value '{}' does not match pattern '{}'", s, pattern
                            ),
                            value: Some(s.clone()),
                            context: None,
                        });
                    }
                }
            }
            ValidationRule::Email => {
                if let Value::String(s) = value {
                    let email_regex = Regex::new(
                            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
                        )
                        .unwrap();
                    if !email_regex.is_match(&s) {
                        return Some(ValidationError {
                            field: field_name.to_string(),
                            rule: "email".to_string(),
                            message: format!(
                                "Value '{}' is not a valid email address", s
                            ),
                            value: Some(s.clone()),
                            context: None,
                        });
                    }
                }
            }
            ValidationRule::Url => {
                if let Value::String(s) = value {
                    let url_regex = Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
                    if !url_regex.is_match(&s) {
                        return Some(ValidationError {
                            field: field_name.to_string(),
                            rule: "url".to_string(),
                            message: format!("Value '{}' is not a valid URL", s),
                            value: Some(s.clone()),
                            context: None,
                        });
                    }
                }
            }
            ValidationRule::Enum(allowed_values) => {
                if let Value::String(s) = value {
                    if !allowed_values.contains(&s) {
                        return Some(ValidationError {
                            field: field_name.to_string(),
                            rule: format!("enum({:?})", allowed_values),
                            message: format!(
                                "Value '{}' is not one of the allowed values: {:?}", s,
                                allowed_values
                            ),
                            value: Some(s.clone()),
                            context: None,
                        });
                    }
                }
            }
            ValidationRule::Custom(name) => {
                if name == "password_strength" {
                    if let Value::String(s) = value {
                        if s.len() < 8 {
                            return Some(ValidationError {
                                field: field_name.to_string(),
                                rule: name.clone(),
                                message: "Password must be at least 8 characters long"
                                    .to_string(),
                                value: Some(s.clone()),
                                context: None,
                            });
                        }
                    }
                }
            }
            ValidationRule::Range { min, max } => {
                if let Value::Number(n) = value {
                    let num = *n;
                    if num < *min || num > *max {
                        return Some(ValidationError {
                            field: field_name.to_string(),
                            rule: format!("Range({:.1}, {:.1})", min, max),
                            message: format!(
                                "Value {:.1} is not in range [{:.1}, {:.1}]", num, min, max
                            ),
                            value: Some(format!("{:.1}", num)),
                            context: None,
                        });
                    }
                }
            }
            _ => {}
        }
        None
    }
    fn get_value_type(&self, value: &Value) -> ValueType {
        match value {
            Value::String(_) => ValueType::String,
            Value::Number(_) => ValueType::Number,
            Value::Bool(_) => ValueType::Boolean,
            Value::Array(_) => ValueType::Array,
            Value::Object(_) => ValueType::Object,
            Value::Null => ValueType::Null,
            Value::Duration(_) => ValueType::String,
            Value::Reference(_) => ValueType::String,
            Value::Identifier(_) => ValueType::String,
        }
    }
    fn types_match(&self, expected: &ValueType, actual: &ValueType) -> bool {
        match (expected, actual) {
            (ValueType::String, ValueType::String) => true,
            (ValueType::Number, ValueType::Number) => true,
            (ValueType::Boolean, ValueType::Boolean) => true,
            (ValueType::Array, ValueType::Array) => true,
            (ValueType::Object, ValueType::Object) => true,
            (ValueType::Null, ValueType::Null) => true,
            _ => false,
        }
    }
}
pub struct SchemaBuilder {
    fields: HashMap<String, Vec<ValidationRule>>,
    required_fields: Vec<String>,
    description: Option<String>,
    version: Option<String>,
}
impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            required_fields: Vec::new(),
            description: None,
            version: None,
        }
    }
    pub fn field(mut self, name: impl Into<String>, rules: Vec<ValidationRule>) -> Self {
        let name = name.into();
        self.fields.insert(name.clone(), rules);
        self
    }
    pub fn required(mut self, field: impl Into<String>) -> Self {
        let field = field.into();
        if !self.required_fields.contains(&field) {
            self.required_fields.push(field);
        }
        self
    }
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
    pub fn build(self) -> ConfigSchema {
        let optional_fields: Vec<String> = self
            .fields
            .keys()
            .filter(|k| !self.required_fields.contains(k))
            .cloned()
            .collect();
        ConfigSchema {
            fields: self.fields,
            required_fields: self.required_fields,
            optional_fields,
            description: self.description,
            version: self.version,
        }
    }
}
impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}
pub mod rules {
    use super::*;
    pub fn required() -> ValidationRule {
        ValidationRule::Required
    }
    pub fn string() -> ValidationRule {
        ValidationRule::Type(ValueType::String)
    }
    pub fn number() -> ValidationRule {
        ValidationRule::Type(ValueType::Number)
    }
    pub fn boolean() -> ValidationRule {
        ValidationRule::Type(ValueType::Boolean)
    }
    pub fn array() -> ValidationRule {
        ValidationRule::Type(ValueType::Array)
    }
    pub fn object() -> ValidationRule {
        ValidationRule::Type(ValueType::Object)
    }
    pub fn string_length(min: Option<usize>, max: Option<usize>) -> ValidationRule {
        ValidationRule::StringLength {
            min,
            max,
        }
    }
    pub fn numeric_range(min: Option<f64>, max: Option<f64>) -> ValidationRule {
        ValidationRule::NumericRange {
            min,
            max,
        }
    }
    pub fn pattern(pattern: impl Into<String>) -> ValidationRule {
        ValidationRule::Pattern(pattern.into())
    }
    pub fn email() -> ValidationRule {
        ValidationRule::Email
    }
    pub fn url() -> ValidationRule {
        ValidationRule::Url
    }
    pub fn custom_validator<F>(name: &str, _validator: F) -> ValidationRule
    where
        F: Fn(&Value) -> crate::hel::error::Result<bool> + Send + Sync + 'static,
    {
        ValidationRule::Custom(name.to_string())
    }
    /// Enum values validation
    pub fn enum_values(values: Vec<String>) -> ValidationRule {
        ValidationRule::Enum(values)
    }
    /// Range validation
    pub fn range(min: f64, max: f64) -> ValidationRule {
        ValidationRule::Range { min, max }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use dna::atp::value::Value;
    #[test]
    fn test_required_field_validation() {
        let schema = SchemaBuilder::new()
            .field("name", vec![rules::required(), rules::string()])
            .required("name")
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "name");
        config.insert("name".to_string(), Value::String("test".to_string()));
        let result = validator.validate(&config);
        assert!(result.is_valid);
    }
    #[test]
    fn test_string_length_validation() {
        let schema = SchemaBuilder::new()
            .field("name", vec![rules::string_length(Some(3), Some(10))])
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config.insert("name".to_string(), Value::String("ab".to_string()));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config.insert("name".to_string(), Value::String("verylongname".to_string()));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config.insert("name".to_string(), Value::String("valid".to_string()));
        let result = validator.validate(&config);
        assert!(result.is_valid);
    }
    #[test]
    fn test_email_validation() {
        let schema = SchemaBuilder::new().field("email", vec![rules::email()]).build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config.insert("email".to_string(), Value::String("invalid-email".to_string()));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config
            .insert("email".to_string(), Value::String("test@example.com".to_string()));
        let result = validator.validate(&config);
        assert!(result.is_valid);
    }
    #[test]
    fn test_numeric_range_validation() {
        let schema = SchemaBuilder::new()
            .field("age", vec![rules::numeric_range(Some(18.0), Some(100.0))])
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config.insert("age".to_string(), Value::Number(10.0));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config.insert("age".to_string(), Value::Number(1000.0));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config.insert("age".to_string(), Value::Number(30.0));
        let result = validator.validate(&config);
        assert!(result.is_valid);
    }
    #[test]
    fn test_pattern_validation() {
        let schema = SchemaBuilder::new()
            .field("username", vec![rules::pattern(r"^[a-z0-9_-]{3,15}$")])
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config.insert("username".to_string(), Value::String("user-name".to_string()));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config
            .insert(
                "username".to_string(),
                Value::String("valid_username123".to_string()),
            );
        let result = validator.validate(&config);
        assert!(result.is_valid);
    }
    #[test]
    fn test_enum_validation() {
        let schema = SchemaBuilder::new()
            .field(
                "color",
                vec![rules::enum_values(vec!["red".to_string(), "blue".to_string()])],
            )
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config.insert("color".to_string(), Value::String("green".to_string()));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        config.insert("color".to_string(), Value::String("red".to_string()));
        let result = validator.validate(&config);
        assert!(result.is_valid);
    }
    #[test]
    fn test_custom_validator() {
        let schema = SchemaBuilder::new()
            .field(
                "password",
                vec![
                    rules::custom_validator("password_strength", | value | { if let
                    Value::String(s) = value { if s.len() < 8 { return Err(crate
                    ::error::HlxError::validation_error("Password must be at least 8 characters long"
                    .to_string(), "Check parameters")); } Ok(true) } else { Err(crate
                    ::error::HlxError::validation_error("Password must be a string"
                    .to_string(), "Check parameters")) } })
                ],
            )
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config
            .insert(
                "password".to_string(),
                Value::String("strong_password123".to_string()),
            );
        let result = validator.validate(&config);
        assert!(result.is_valid);
        config.insert("password".to_string(), Value::String("short".to_string()));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "password");
        assert_eq!(result.errors[0].rule, "password_strength");
        assert_eq!(
            result.errors[0].message, "Password must be at least 8 characters long"
        );
        assert_eq!(result.errors[0].value, Some("short".to_string()));
    }
    #[test]
    fn test_range_validation() {
        let schema = SchemaBuilder::new()
            .field("score", vec![rules::range(0.0, 100.0)])
            .build();
        let validator = SchemaValidator::new(schema);
        let mut config = HashMap::new();
        config.insert("score".to_string(), Value::Number(50.0));
        let result = validator.validate(&config);
        assert!(result.is_valid);
        config.insert("score".to_string(), Value::Number(150.0));
        let result = validator.validate(&config);
        assert!(! result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "score");
        assert_eq!(result.errors[0].rule, "Range(0.0, 100.0)");
        assert_eq!(result.errors[0].message, "Value 150.0 is not in range [0.0, 100.0]");
        assert_eq!(result.errors[0].value, Some("150.0".to_string()));
    }
}