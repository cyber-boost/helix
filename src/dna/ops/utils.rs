use crate::dna::hel::error::HlxError;
use crate::dna::atp::value::Value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub fn parse_params(params: &str) -> Result<HashMap<String, Value>, HlxError> {
    let json_str = params.trim_matches('"').trim_matches('\'');
    let json_str = json_str.replace("\\\"", "\"").replace("\\'", "'");
    if json_str.is_empty() {
        return Ok(HashMap::new());
    }
    match serde_json::from_str::<JsonValue>(&json_str) {
        Ok(JsonValue::Object(obj)) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_value_to_value(&v));
            }
            Ok(map)
        }
        _ => Err(HlxError::invalid_parameters("unknown", params)),
    }
}
pub fn json_value_to_value(json_value: &JsonValue) -> Value {
    match json_value {
        JsonValue::String(s) => {
            if s.is_empty() {
                Value::String("".to_string())
            } else {
                Value::String(s.clone())
            }
        }
        JsonValue::Number(n) => {
            if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::String(n.to_string())
            }
        }
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Array(arr) => {
            let values: Vec<Value> = arr
                .iter()
                .map(|v| json_value_to_value(v))
                .collect();
            Value::Array(values)
        }
        JsonValue::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_value_to_value(v));
            }
            Value::Object(map)
        }
        JsonValue::Null => Value::Null,
    }
}
pub fn value_to_json_value(value: &Value) -> JsonValue {
    match value {
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Number(n) => {
            JsonValue::Number(
                serde_json::Number::from_f64(*n)
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            )
        }
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Array(arr) => {
            let values: Vec<JsonValue> = arr
                .iter()
                .map(|v| value_to_json_value(v))
                .collect();
            JsonValue::Array(values)
        }
        Value::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (k, v) in obj {
                map.insert(k.clone(), value_to_json_value(v));
            }
            JsonValue::Object(map)
        }
        Value::Null => JsonValue::Null,
        Value::Duration(d) => JsonValue::String(format!("{} {:?}", d.value, d.unit)),
        Value::Reference(r) => JsonValue::String(format!("@{}", r)),
        Value::Identifier(i) => JsonValue::String(i.clone()),
        _ => JsonValue::Null,
    }
}

pub fn value_to_json(value: &Value) -> serde_json::Value {
match value {
    Value::String(s) => serde_json::Value::String(s.clone()),
    Value::Number(n) => {
        serde_json::Value::Number(
            serde_json::Number::from_f64(*n)
                .unwrap_or_else(|| serde_json::Number::from(0)),
        )
    }
    Value::Bool(b) => serde_json::Value::Bool(*b),
    Value::Array(arr) => {
        serde_json::Value::Array(arr.iter().map(value_to_json).collect())
    }
    Value::Object(obj) => {
        serde_json::Value::Object(
            obj.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect(),
        )
    }
    Value::Null => serde_json::Value::Null,
    Value::Duration(d) => serde_json::Value::String(format!("{} {:?}", d.value, d.unit)),
    Value::Reference(r) => serde_json::Value::String(format!("@{}", r)),
    Value::Identifier(i) => serde_json::Value::String(i.clone()),
}
}
pub fn json_to_value(json_value: &serde_json::Value) -> Value {
match json_value {
    serde_json::Value::String(s) => Value::String(s.clone()),
    serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
    serde_json::Value::Bool(b) => Value::Bool(*b),
    serde_json::Value::Array(arr) => {
        Value::Array(arr.iter().map(json_to_value).collect())
    }
    serde_json::Value::Object(obj) => {
        Value::Object(
            obj.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect(),
        )
    }
    serde_json::Value::Null => Value::Null,
}
}