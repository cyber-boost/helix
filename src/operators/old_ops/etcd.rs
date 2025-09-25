//! etcd Key-Value Store Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdConfig {
    pub endpoints: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub connect_timeout: u64,
    pub request_timeout: u64,
    pub enable_ssl: bool,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://localhost:2379".to_string()],
            username: None,
            password: None,
            connect_timeout: 5,
            request_timeout: 10,
            enable_ssl: false,
            ca_cert: None,
            client_cert: None,
            client_key: None,
        }
    }
}

#[derive(Debug, Default)]
struct EtcdMetrics {
    get_operations: u64,
    put_operations: u64,
    delete_operations: u64,
    watch_operations: u64,
    lock_operations: u64,
    lease_operations: u64,
    avg_response_time: f64,
}

pub struct EtcdOperator {
    config: EtcdConfig,
    metrics: Arc<Mutex<EtcdMetrics>>,
    storage: Arc<RwLock<HashMap<String, String>>>, // Mock storage
    watchers: Arc<RwLock<HashMap<String, bool>>>,   // Mock watchers
    locks: Arc<RwLock<HashMap<String, String>>>,    // Mock locks
}

impl EtcdOperator {
    pub async fn new(config: EtcdConfig) -> Result<Self, HlxError> {

        Ok(Self {
            config,
            metrics: Arc::new(Mutex::new(EtcdMetrics::default())),
            storage: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, HlxError> {

        let storage = self.storage.read().await;
        let value = storage.get(key).cloned();

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.get_operations += 1;
        }

        Ok(value)
    }

    pub async fn put(&self, key: &str, value: &str, ttl: Option<u64>) -> Result<(), HlxError> {

        let mut storage = self.storage.write().await;
        storage.insert(key.to_string(), value.to_string());

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.put_operations += 1;
        }

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<bool, HlxError> {

        let mut storage = self.storage.write().await;
        let existed = storage.remove(key).is_some();

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.delete_operations += 1;
        }

        Ok(existed)
    }

    pub async fn get_prefix(&self, prefix: &str) -> Result<HashMap<String, String>, HlxError> {

        let storage = self.storage.read().await;
        let mut results = HashMap::new();

        for (key, value) in storage.iter() {
            if key.starts_with(prefix) {
                results.insert(key.clone(), value.clone());
            }
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.get_operations += 1;
        }

        Ok(results)
    }

    pub async fn delete_prefix(&self, prefix: &str) -> Result<u64, HlxError> {

        let mut storage = self.storage.write().await;
        let keys_to_remove: Vec<String> = storage.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();

        let deleted_count = keys_to_remove.len() as u64;
        for key in keys_to_remove {
            storage.remove(&key);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.delete_operations += 1;
        }

        Ok(deleted_count)
    }

    pub async fn watch(&self, key: &str) -> Result<String, HlxError> {

        let watch_id = format!("watch-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        {
            let mut watchers = self.watchers.write().await;
            watchers.insert(watch_id.clone(), true);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.watch_operations += 1;
        }

        Ok(watch_id)
    }

    pub async fn cancel_watch(&self, watch_id: &str) -> Result<(), HlxError> {

        let mut watchers = self.watchers.write().await;
        watchers.remove(watch_id);

        Ok(())
    }

    pub async fn acquire_lock(&self, key: &str, ttl: Option<u64>) -> Result<String, HlxError> {

        let lock_id = format!("lock-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        {
            let mut locks = self.locks.write().await;
            if locks.contains_key(key) {
                return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                    operation: "Acquire Lock".to_string(),
                    message: format!("Lock already exists for key: {}", key),
                });
            }
            locks.insert(key.to_string(), lock_id.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.lock_operations += 1;
        }

        Ok(lock_id)
    }

    pub async fn release_lock(&self, key: &str, lock_id: &str) -> Result<(), HlxError> {

        let mut locks = self.locks.write().await;
        if let Some(existing_id) = locks.get(key) {
            if existing_id == lock_id {
                locks.remove(key);
                Ok(())
            } else {
                Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                    operation: "Release Lock".to_string(),
                    message: "Lock ID does not match".to_string(),
                })
            }
        } else {
            Err(HlxError::NotFoundError {
                resource: "Lock".to_string(),
                identifier: key.to_string(),
            })
        }
    }

    pub async fn create_lease(&self, ttl: u64) -> Result<String, HlxError> {

        let lease_id = format!("lease-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.lease_operations += 1;
        }

        Ok(lease_id)
    }

    pub async fn renew_lease(&self, lease_id: &str) -> Result<u64, HlxError> {

        // Mock renewed TTL
        let new_ttl = 60u64;

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.lease_operations += 1;
        }

        Ok(new_ttl)
    }

    pub async fn revoke_lease(&self, lease_id: &str) -> Result<(), HlxError> {

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.lease_operations += 1;
        }

        Ok(())
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("get_operations".to_string(), Value::Number(metrics.get_operations as f64));
        result.insert("put_operations".to_string(), Value::Number(metrics.put_operations as f64));
        result.insert("delete_operations".to_string(), Value::Number(metrics.delete_operations as f64));
        result.insert("watch_operations".to_string(), Value::Number(metrics.watch_operations as f64));
        result.insert("lock_operations".to_string(), Value::Number(metrics.lock_operations as f64));
        result.insert("lease_operations".to_string(), Value::Number(metrics.lease_operations as f64));
        result.insert("avg_response_time_ms".to_string(), Value::Number(metrics.avg_response_time));

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for EtcdOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "get" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let value = self.get(&key).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("value".to_string(), value.map(Value::String).unwrap_or(Value::Null));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "put" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let value = params_map.get("value").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("value".to_string()),
                        message: "Missing value".to_string(),
                    })?;

                let ttl = params_map.get("ttl").and_then(|v| v.as_number()).map(|n| n as u64);

                self.put(&key, &value, ttl).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("value".to_string(), Value::String(value.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let existed = self.delete(&key).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("existed".to_string(), Value::Boolean(existed));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_prefix" => {
                let prefix = params_map.get("prefix").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("prefix".to_string()),
                        message: "Missing prefix".to_string(),
                    })?;

                let results = self.get_prefix(&prefix).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("prefix".to_string(), Value::String(prefix.to_string(.to_string())));
                    map.insert("count".to_string(), Value::Number(results.len() as f64));
                    map.insert("results".to_string(), Value::Object(
                        results.into_iter().map(|(k, v)| (k, Value::String(v.to_string()))).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_prefix" => {
                let prefix = params_map.get("prefix").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("prefix".to_string()),
                        message: "Missing prefix".to_string(),
                    })?;

                let deleted_count = self.delete_prefix(&prefix).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("prefix".to_string(), Value::String(prefix.to_string(.to_string())));
                    map.insert("deleted_count".to_string(), Value::Number(deleted_count as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "watch" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let watch_id = self.watch(&key).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("watch_id".to_string(), Value::String(watch_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "cancel_watch" => {
                let watch_id = params_map.get("watch_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("watch_id".to_string()),
                        message: "Missing watch ID".to_string(),
                    })?;

                self.cancel_watch(&watch_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("watch_id".to_string(), Value::String(watch_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "acquire_lock" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let ttl = params_map.get("ttl").and_then(|v| v.as_number()).map(|n| n as u64);

                let lock_id = self.acquire_lock(&key, ttl).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("lock_id".to_string(), Value::String(lock_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "release_lock" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let lock_id = params_map.get("lock_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("lock_id".to_string()),
                        message: "Missing lock ID".to_string(),
                    })?;

                self.release_lock(&key, &lock_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("lock_id".to_string(), Value::String(lock_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "create_lease" => {
                let ttl = params_map.get("ttl").and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("ttl".to_string()),
                        message: "Missing TTL".to_string(),
                    })? as u64;

                let lease_id = self.create_lease(ttl).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("lease_id".to_string(), Value::String(lease_id.to_string(.to_string())));
                    map.insert("ttl".to_string(), Value::Number(ttl as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "renew_lease" => {
                let lease_id = params_map.get("lease_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("lease_id".to_string()),
                        message: "Missing lease ID".to_string(),
                    })?;

                let new_ttl = self.renew_lease(&lease_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("lease_id".to_string(), Value::String(lease_id.to_string(.to_string())));
                    map.insert("new_ttl".to_string(), Value::Number(new_ttl as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "revoke_lease" => {
                let lease_id = params_map.get("lease_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("lease_id".to_string()),
                        message: "Missing lease ID".to_string(),
                    })?;

                self.revoke_lease(&lease_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("lease_id".to_string(), Value::String(lease_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "etcd".to_string(),
                params: format!("Unknown etcd operation: {}", operator),
            }),
        }
    }
} 