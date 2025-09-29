use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::atp::types::Duration;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Null,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Null,
    Duration(Duration),
    Reference(String),
    Identifier(String),
}

// Add Default implementation for Value
impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::String(_) => ValueType::String,
            Value::Number(_) => ValueType::Number,
            Value::Bool(_) => ValueType::Boolean,
            Value::Array(_) => ValueType::Array,
            Value::Object(_) => ValueType::Object,
            Value::Null => ValueType::Null,
            Value::Duration(_) => ValueType::String, // Treat as string for now
            Value::Reference(_) => ValueType::String,
            Value::Identifier(_) => ValueType::String,
        }
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number()
    }
    pub fn as_str(&self) -> Option<&str> {
        self.as_string()
    }
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(obj) => obj.get(key),
            _ => None,
        }
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Object(obj) => obj.get_mut(key),
            _ => None,
        }
    }
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_string()
    }
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_number()
    }
    pub fn get_boolean(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_boolean()
    }
    pub fn get_array(&self, key: &str) -> Option<&[Value]> {
        self.get(key)?.as_array()
    }
    pub fn get_object(&self, key: &str) -> Option<&HashMap<String, Value>> {
        self.get(key)?.as_object()
    }
    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Null => "null".to_string(),
            Value::Duration(d) => format!("{} {:?}", d.value, d.unit),
            Value::Reference(r) => format!("@{}", r),
            Value::Identifier(i) => i.clone(),
        }
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
    pub fn from_json(json_value: serde_json::Value) -> Self {
        match json_value {
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| Value::from_json(v.clone())).collect())
            }
            serde_json::Value::Object(obj) => {
                Value::Object(
                    obj
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::from_json(v.clone())))
                        .collect(),
                )
            }
            serde_json::Value::Null => Value::Null,
        }
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s.to_string())
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}
impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n as f64)
    }
}
impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}
impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<Vec<Value>> for Value {
    fn from(arr: Vec<Value>) -> Self {
        Value::Array(arr)
    }
}
impl From<HashMap<String, Value>> for Value {
    fn from(obj: HashMap<String, Value>) -> Self {
        Value::Object(obj)
    }
}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_value_type() {
        assert_eq!(Value::String("test".to_string()).value_type(), ValueType::String);
        assert_eq!(Value::Number(42.0).value_type(), ValueType::Number);
        assert_eq!(Value::Bool(true).value_type(), ValueType::Boolean);
        assert_eq!(Value::Array(vec![]).value_type(), ValueType::Array);
        assert_eq!(Value::Object(HashMap::new()).value_type(), ValueType::Object);
        assert_eq!(Value::Null.value_type(), ValueType::Null);
    }
    #[test]
    fn test_type_checks() {
        let string_val = Value::String("test".to_string());
        assert!(string_val.is_string());
        assert!(! string_val.is_number());
        let number_val = Value::Number(42.0);
        assert!(number_val.is_number());
        assert!(! number_val.is_string());
    }
    #[test]
    fn test_conversions() {
        let string_val = Value::from("test");
        assert_eq!(string_val, Value::String("test".to_string()));
        let number_val = Value::from(42);
        assert_eq!(number_val, Value::Number(42.0));
        let bool_val = Value::from(true);
        assert_eq!(bool_val, Value::Bool(true));
    }
    #[test]
    fn test_object_access() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::String("test".to_string()));
        obj.insert("count".to_string(), Value::Number(42.0));
        let value = Value::Object(obj);
        assert_eq!(value.get_string("name"), Some("test"));
        assert_eq!(value.get_number("count"), Some(42.0));
        assert_eq!(value.get_string("missing"), None);
    }
    #[test]
    fn test_to_string() {
        assert_eq!(Value::String("test".to_string()).to_string(), "test");
        assert_eq!(Value::Number(42.0).to_string(), "42");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Null.to_string(), "null");
    }
}